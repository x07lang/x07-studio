use anyhow::{anyhow, bail};
use loom_adapters::x07_cli::{CliAdapter, X07JsonOptions};
use loom_types::api::{PkgCandidate, PkgProvidesResult};
use serde_json::Value;

pub async fn run(
    module_id: &str,
    adapter: &CliAdapter,
) -> anyhow::Result<(PkgProvidesResult, loom_adapters::x07_cli::ExecutedBinding)> {
    validate_module_id(module_id)?;
    let executed = adapter
        .execute_x07_json(
            "pkg.provides",
            "x07/package",
            vec![
                "pkg".to_string(),
                "provides".to_string(),
                module_id.to_string(),
                "--project".to_string(),
                "x07.json".to_string(),
            ],
            vec!["x07.lock.json".to_string()],
            "Resolve the package candidates that provide an X07 module id.",
            X07JsonOptions::stdout(Some(30)),
        )
        .await?;
    let raw = executed.report_json.clone().unwrap_or_else(|| {
        serde_json::json!({
            "stdout": executed.execution.stdout,
            "stderr": executed.execution.stderr,
            "exit_code": executed.execution.exit_code
        })
    });
    Ok((
        PkgProvidesResult {
            schema_version: "x07.studio.pkg_provides_result@0.1.0".to_string(),
            module_id: module_id.to_string(),
            candidates: candidates_from_raw(module_id, &raw),
        },
        executed,
    ))
}

pub fn candidates_from_raw(module_id: &str, raw: &Value) -> Vec<PkgCandidate> {
    let mut values = Vec::new();
    collect_candidates(raw, &mut values);
    values
        .into_iter()
        .map(|value| {
            let package = first_string(value, &["package", "name", "package_name"])
                .unwrap_or_else(|| module_id.replace('.', "-"));
            let version = first_string(value, &["version"]).unwrap_or_else(|| "latest".to_string());
            let source = first_string(value, &["source", "registry"])
                .unwrap_or_else(|| "registry".to_string());
            let install_command = first_string(value, &["install_command", "command"])
                .unwrap_or_else(|| format!("x07 pkg add {package}@{version}"));
            PkgCandidate {
                package,
                version,
                source,
                install_command,
            }
        })
        .collect()
}

fn validate_module_id(module_id: &str) -> anyhow::Result<()> {
    let trimmed = module_id.trim();
    if trimmed.is_empty() {
        bail!("module id is required");
    }
    if trimmed.contains('\0')
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Err(anyhow!(
            "module id `{module_id}` is not a valid X07 module id"
        ));
    }
    Ok(())
}

fn collect_candidates<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if first_string(value, &["package", "name", "package_name"]).is_some()
                && (first_string(value, &["version"]).is_some()
                    || first_string(value, &["install_command", "command"]).is_some())
            {
                out.push(value);
                return;
            }
            for (key, child) in map {
                if matches!(key.as_str(), "candidates" | "packages" | "results") {
                    if let Value::Array(items) = child {
                        for item in items {
                            collect_candidates(item, out);
                        }
                        continue;
                    }
                }
                collect_candidates(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_candidates(item, out);
            }
        }
        _ => {}
    }
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
    use super::candidates_from_raw;

    #[test]
    fn reads_candidates_fixture() {
        let raw = serde_json::json!({"candidates":[{"package":"ext-text","version":"0.5.0","source":"registry"}]});
        let candidates = candidates_from_raw("text.normalize_v1", &raw);
        assert_eq!(candidates[0].install_command, "x07 pkg add ext-text@0.5.0");
    }
}
