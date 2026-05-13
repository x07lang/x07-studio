use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};
use loom_adapters::x07_cli::{CliAdapter, X07JsonOptions};
use loom_types::api::{PbtCounterexample, PbtRound, ProofEvidenceCitation, QuickfixRecord};
use serde_json::Value;
use uuid::Uuid;

pub async fn run(
    root: &Utf8Path,
    session_id: Uuid,
    adapter: &CliAdapter,
) -> anyhow::Result<(PbtRound, loom_adapters::x07_cli::ExecutedBinding)> {
    let mut args = vec!["test".to_string(), "--pbt".to_string()];
    if root.join("gen/xtal/tests.json").is_file() {
        args.extend(["--manifest".to_string(), "gen/xtal/tests.json".to_string()]);
    } else if root.join("tests/tests.json").is_file() {
        args.extend(["--manifest".to_string(), "tests/tests.json".to_string()]);
    }
    let executed = adapter
        .execute_x07_json(
            "pbt.run",
            "x07/test",
            args,
            vec!["target/x07test/pbt".to_string()],
            "Run x07 property-based tests.",
            X07JsonOptions::report_file(Some(120)),
        )
        .await?;
    let raw = executed.report_json.clone().unwrap_or_else(|| {
        serde_json::json!({
            "stdout": executed.execution.stdout,
            "stderr": executed.execution.stderr,
            "exit_code": executed.execution.exit_code
        })
    });
    let round = PbtRound {
        schema_version: "x07.studio.pbt_round@0.1.0".to_string(),
        session_id,
        started_at: executed.execution.started_at.clone(),
        finished_at: Some(executed.execution.finished_at.clone()),
        properties_run: first_u32(&raw, &["properties_run", "property_count", "tests_run"])
            .unwrap_or_else(|| counterexamples_from_raw(&raw).len() as u32),
        counterexamples: counterexamples_from_raw(&raw),
        raw,
    };
    Ok((round, executed))
}

pub async fn regression_from(
    root: &Utf8Path,
    _session_id: Uuid,
    repro_id: &str,
    adapter: &CliAdapter,
) -> anyhow::Result<(QuickfixRecord, loom_adapters::x07_cli::ExecutedBinding)> {
    let repro_path =
        find_repro(root, repro_id).ok_or_else(|| anyhow!("unknown PBT repro `{repro_id}`"))?;
    let rel = repro_path
        .strip_prefix(root)
        .unwrap_or(repro_path.as_path())
        .to_string();
    let executed = adapter
        .execute_x07_json(
            "pbt.regression_from",
            "x07/fix",
            vec![
                "fix".to_string(),
                "--from-pbt".to_string(),
                rel.clone(),
                "--write".to_string(),
            ],
            vec!["tests/".to_string(), rel.clone()],
            "Convert a PBT counterexample into a deterministic regression test.",
            X07JsonOptions::report_file(Some(60)),
        )
        .await?;
    let record = QuickfixRecord {
        schema_version: "x07.studio.quickfix_record@0.1.0".to_string(),
        diagnostic_code: repro_id.to_string(),
        severity: "info".to_string(),
        summary: format!("Locked PBT counterexample `{repro_id}` as a regression test."),
        patch_ast: executed.report_json.clone().unwrap_or_else(|| {
            serde_json::json!({
                "repro": rel,
                "exit_code": executed.execution.exit_code,
                "stdout": executed.execution.stdout,
                "stderr": executed.execution.stderr
            })
        }),
        citations: vec![ProofEvidenceCitation {
            kind: "pbt_repro".to_string(),
            file: rel,
            region: Some("counterexample".to_string()),
        }],
        before_snippet: None,
        after_snippet: None,
    };
    Ok((crate::quickfix::with_snippets(root, record), executed))
}

pub fn counterexamples_from_raw(raw: &Value) -> Vec<PbtCounterexample> {
    let mut values = Vec::new();
    collect_counterexamples(raw, &mut values);
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| PbtCounterexample {
            repro_id: first_string(value, &["repro_id", "id"])
                .unwrap_or_else(|| format!("pbt-repro-{}", index + 1)),
            property: first_string(value, &["property", "test", "name"])
                .unwrap_or_else(|| "property".to_string()),
            shrunk_input: first_value(value, &["shrunk_input", "input", "counterexample"])
                .cloned()
                .unwrap_or(Value::Null),
            repro_path: first_string(value, &["repro_path", "path"]).unwrap_or_default(),
        })
        .collect()
}

fn collect_counterexamples<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if first_value(value, &["counterexample", "shrunk_input", "repro_path"]).is_some() {
                out.push(value);
                return;
            }
            for (key, child) in map {
                if matches!(key.as_str(), "counterexamples" | "failures" | "repros") {
                    if let Value::Array(items) = child {
                        for item in items {
                            collect_counterexamples(item, out);
                        }
                        continue;
                    }
                }
                collect_counterexamples(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_counterexamples(item, out);
            }
        }
        _ => {}
    }
}

fn find_repro(root: &Utf8Path, repro_id: &str) -> Option<Utf8PathBuf> {
    let rel = Utf8Path::new(repro_id);
    if !repro_id.contains('\0')
        && !rel.is_absolute()
        && !rel.components().any(|part| part.as_str() == "..")
        && root.join(rel).is_file()
    {
        return Some(root.join(rel));
    }
    for base in [
        ".x07/cache/pbt/repros",
        "target/x07test/pbt",
        "target/xtal/pbt",
    ] {
        let dir = root.join(base);
        let mut found = None;
        visit_files(dir.as_path(), &mut |path| {
            if found.is_some() {
                return;
            }
            let Some(name) = path.file_name() else {
                return;
            };
            if name == repro_id
                || name == format!("{repro_id}.json")
                || path.as_str().contains(repro_id)
            {
                found = Some(path.to_owned());
            }
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

fn visit_files(dir: &Utf8Path, f: &mut dyn FnMut(&Utf8Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        if path.is_dir() {
            visit_files(path.as_path(), f);
        } else {
            f(path.as_path());
        }
    }
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    first_value(value, keys)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn first_u32(value: &Value, keys: &[&str]) -> Option<u32> {
    first_value(value, keys)
        .and_then(Value::as_u64)
        .and_then(|num| u32::try_from(num).ok())
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
    use super::counterexamples_from_raw;

    #[test]
    fn reads_counterexample_fixture() {
        let raw = serde_json::json!({
            "properties_run": 47,
            "counterexamples": [{
                "repro_id": "r1",
                "property": "stable",
                "shrunk_input": [1,2],
                "repro_path": ".x07/cache/pbt/repros/r1.json"
            }]
        });
        let examples = counterexamples_from_raw(&raw);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].repro_id, "r1");
    }
}
