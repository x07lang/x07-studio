use camino::Utf8Path;
use loom_types::api::{ProofEvidenceCitation, QuickfixRecord};
use serde_json::Value;

pub fn for_incident(root: &Utf8Path, incident_id: &str) -> QuickfixRecord {
    let incident = incident_json(root, incident_id).unwrap_or(Value::Null);
    let diagnostic_code = first_string(&incident, &["diagnostic_code", "code", "kind"])
        .unwrap_or_else(|| "X07-INCIDENT".to_string());
    let severity = first_string(&incident, &["severity"]).unwrap_or_else(|| "warning".to_string());
    let summary = first_string(&incident, &["summary", "message", "error"]).unwrap_or_else(|| {
        format!("Incident `{incident_id}` can be converted into a bounded x07 quickfix review.")
    });
    let patch_ast = first_value(&incident, &["patch", "patch_ast", "patchset"]).cloned()
        .or_else(|| latest_patchset(root))
        .unwrap_or_else(|| {
            serde_json::json!({
                "schema_version": "x07.patchset@0.1.0",
                "operations": [],
                "note": "No deterministic patch was emitted yet; run incident repair to materialize one."
            })
        });
    QuickfixRecord {
        schema_version: "x07.studio.quickfix_record@0.1.0".to_string(),
        diagnostic_code,
        severity,
        summary,
        patch_ast,
        citations: vec![ProofEvidenceCitation {
            kind: "incident".to_string(),
            file: format!(".x07-wasm/incidents/{incident_id}"),
            region: Some("run.report.json".to_string()),
        }],
    }
}

fn incident_json(root: &Utf8Path, incident_id: &str) -> Option<Value> {
    let candidates = [
        root.join(format!(".x07-wasm/incidents/{incident_id}/run.report.json")),
        root.join(format!(
            "target/xtal/violations/{incident_id}/run.report.json"
        )),
        root.join(format!("target/xtal/ingest/{incident_id}.json")),
    ];
    candidates
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn latest_patchset(root: &Utf8Path) -> Option<Value> {
    let mut latest: Option<(std::time::SystemTime, Value)> = None;
    visit_files(root.join("target/xtal/repair").as_std_path(), &mut |path| {
        if path.file_name().and_then(|name| name.to_str()) != Some("patchset.json") {
            return;
        }
        let Ok(metadata) = std::fs::metadata(path) else {
            return;
        };
        let Ok(modified) = metadata.modified() else {
            return;
        };
        let Some(value) = std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
        else {
            return;
        };
        if latest
            .as_ref()
            .map(|(old, _)| modified > *old)
            .unwrap_or(true)
        {
            latest = Some((modified, value));
        }
    });
    latest.map(|(_, value)| value)
}

fn visit_files(dir: &std::path::Path, f: &mut impl FnMut(&std::path::Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, f);
        } else {
            f(&path);
        }
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
    use super::for_incident;
    use uuid::Uuid;

    #[test]
    fn quickfix_reads_incident_diagnostic_code() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-quickfix-{}", Uuid::new_v4()));
        let dir = root.join(".x07-wasm/incidents/i1");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("run.report.json"),
            r#"{"diagnostic_code":"E123","severity":"error","summary":"bad input"}"#,
        )
        .expect("incident");

        let record = for_incident(root.as_path(), "i1");

        assert_eq!(record.diagnostic_code, "E123");
        assert_eq!(record.severity, "error");
        assert!(record.summary.contains("bad input"));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn quickfix_uses_incident_kind_when_code_is_missing() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-quickfix-kind-{}", Uuid::new_v4()));
        let dir = root.join(".x07-wasm/incidents/i2");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("run.report.json"),
            r#"{"kind":"runtime_violation"}"#,
        )
        .expect("incident");

        let record = for_incident(root.as_path(), "i2");

        assert_eq!(record.diagnostic_code, "runtime_violation");
        std::fs::remove_dir_all(root).ok();
    }
}
