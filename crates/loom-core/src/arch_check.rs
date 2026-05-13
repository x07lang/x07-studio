use loom_adapters::x07_cli::{CliAdapter, X07JsonOptions};
use loom_types::api::{ArchCheckReport, ArchViolation};
use serde_json::Value;

pub async fn run(
    adapter: &CliAdapter,
) -> anyhow::Result<(ArchCheckReport, loom_adapters::x07_cli::ExecutedBinding)> {
    let executed = adapter
        .execute_x07_json(
            "arch.check",
            "x07/arch",
            vec!["arch".to_string(), "check".to_string()],
            vec!["arch/".to_string()],
            "Check repo-level architecture invariants.",
            X07JsonOptions::report_file(Some(60)),
        )
        .await?;
    let raw = executed.report_json.clone().unwrap_or_else(|| {
        serde_json::json!({
            "stdout": executed.execution.stdout,
            "stderr": executed.execution.stderr,
            "exit_code": executed.execution.exit_code
        })
    });
    let violations = violations_from_raw(&raw);
    Ok((
        ArchCheckReport {
            schema_version: "x07.studio.arch_check_report@0.1.0".to_string(),
            passed: executed.execution.exit_code == Some(0)
                && violations.is_empty()
                && first_bool(&raw, &["passed", "ok"]).unwrap_or(true),
            violations,
            raw,
        },
        executed,
    ))
}

pub fn violations_from_raw(raw: &Value) -> Vec<ArchViolation> {
    let mut values = Vec::new();
    collect_violation_values(raw, &mut values);
    values
        .into_iter()
        .map(|value| ArchViolation {
            rule: first_string(value, &["rule", "id", "code"])
                .unwrap_or_else(|| "arch".to_string()),
            file: first_string(value, &["file", "path"]).unwrap_or_default(),
            summary: first_string(value, &["summary", "message", "title"])
                .unwrap_or_else(|| "Architecture violation".to_string()),
        })
        .collect()
}

fn collect_violation_values<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if first_string(value, &["rule", "code"]).is_some()
                && first_string(value, &["summary", "message"]).is_some()
            {
                out.push(value);
                return;
            }
            for (key, child) in map {
                if matches!(key.as_str(), "violations" | "errors") {
                    if let Value::Array(items) = child {
                        for item in items {
                            collect_violation_values(item, out);
                        }
                        continue;
                    }
                }
                collect_violation_values(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_violation_values(item, out);
            }
        }
        _ => {}
    }
}

fn first_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    first_value(value, keys).and_then(Value::as_bool)
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    first_value(value, keys)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn first_value<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key) {
                    return Some(value);
                }
            }
            map.values().find_map(|value| first_value(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| first_value(value, keys)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::violations_from_raw;

    #[test]
    fn reads_violations_fixture() {
        let raw = serde_json::json!({"violations":[{"rule":"ports","file":"src/main.x07.json","summary":"adapter imported core"}]});
        let violations = violations_from_raw(&raw);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "ports");
    }
}
