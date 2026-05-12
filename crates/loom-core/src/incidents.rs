use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncidentBundle {
    pub id: String,
    pub root_path: String,
    pub summary: String,
    pub at: String,
    pub kind: String,
}

pub fn scan_workspace_incidents(root: &Utf8Path) -> Vec<IncidentBundle> {
    let mut out = Vec::new();
    for base in [
        ".x07-wasm/incidents",
        "target/xtal/violations",
        "target/xtal/ingest",
    ] {
        visit(root.join(base).as_path(), root, &mut out);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);
    out
}

fn visit(path: &Utf8Path, root: &Utf8Path, out: &mut Vec<IncidentBundle>) {
    let Ok(entries) = fs::read_dir(path.as_std_path()) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        if path.is_dir() {
            let report = path.join("run.report.json");
            let violation = path.join("violation.json");
            if report.exists() || violation.exists() {
                out.push(bundle_from_dir(
                    root,
                    path.as_path(),
                    report.as_path(),
                    violation.as_path(),
                ));
            }
            visit(path.as_path(), root, out);
        } else if path.file_name() == Some("run.report.json")
            || path.file_name() == Some("violation.json")
        {
            let dir = path.parent().unwrap_or(path.as_path());
            out.push(bundle_from_dir(root, dir, path.as_path(), path.as_path()));
        }
    }
}

fn bundle_from_dir(
    root: &Utf8Path,
    dir: &Utf8Path,
    report: &Utf8Path,
    violation: &Utf8Path,
) -> IncidentBundle {
    let id = dir
        .file_name()
        .map(str::to_string)
        .unwrap_or_else(|| "incident".to_string());
    let payload = fs::read_to_string(report)
        .ok()
        .or_else(|| fs::read_to_string(violation).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
    let summary = payload
        .as_ref()
        .and_then(|value| value.get("summary").or_else(|| value.get("message")))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Runtime incident bundle detected.")
        .to_string();
    let kind = payload
        .as_ref()
        .and_then(|value| value.get("kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("runtime_violation")
        .to_string();
    let at = payload
        .as_ref()
        .and_then(|value| value.get("at").or_else(|| value.get("generated_at")))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("2000-01-01T00:00:00Z")
        .to_string();
    let root_path = dir
        .strip_prefix(root)
        .map(|path| path.to_string())
        .unwrap_or_else(|_| dir.to_string());
    IncidentBundle {
        id,
        root_path,
        summary,
        at,
        kind,
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::scan_workspace_incidents;

    #[test]
    fn scanner_finds_wasm_incident_reports() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8 temp")
            .join(format!("x07-studio-incidents-{}", Uuid::new_v4()));
        let dir = root.join(".x07-wasm/incidents/demo-iid");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("run.report.json"),
            r#"{"kind":"panic","summary":"payload length was zero","at":"2026-05-12T12:00:00Z"}"#,
        )
        .expect("write");

        let incidents = scan_workspace_incidents(root.as_path());

        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].id, "demo-iid");
        assert!(incidents[0].summary.contains("payload"));
        std::fs::remove_dir_all(root).ok();
    }
}
