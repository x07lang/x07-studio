use camino::Utf8Path;
use loom_adapters::command_runner::now_string;
use loom_types::api::CertificateSummary;
use loom_types::session::SessionSnapshot;
use serde_json::Value;

pub fn summary(root: &Utf8Path, session: &SessionSnapshot) -> CertificateSummary {
    let certificate = read_json(root.join("target/cert/certificate.json").as_path())
        .or_else(|| read_json(root.join("target/xtal/cert/bundle.json").as_path()))
        .unwrap_or(Value::Null);
    let proof_summary = read_json(root.join("target/xtal/cert/summary.json").as_path())
        .or_else(|| read_json(root.join("target/xtal/verify/summary.json").as_path()))
        .or_else(|| latest_op_json(session, "xtal.certify"))
        .unwrap_or(Value::Null);
    let trust_report = read_json(root.join("target/cert/trust-report.json").as_path())
        .or_else(|| read_json(root.join("target/trust/report.json").as_path()))
        .or_else(|| latest_op_json(session, "trust."))
        .unwrap_or(Value::Null);
    CertificateSummary {
        schema_version: "x07.studio.certificate_summary@0.1.0".to_string(),
        session_id: session.session_id,
        profile: string_field(&certificate, &["profile", "trust_profile"])
            .or_else(|| string_field(&trust_report, &["profile", "trust_profile"]))
            .or_else(|| latest_trust_profile_arg(session))
            .unwrap_or_else(|| "local_preview".to_string()),
        operational_entry: string_field(&certificate, &["operational_entry", "entry"])
            .unwrap_or_else(|| "main".to_string()),
        issued_at: string_field(&certificate, &["issued_at", "created_at"])
            .unwrap_or_else(now_string),
        expires_at: string_field(&certificate, &["expires_at"]),
        proof_summary,
        trust_report,
        html_summary_path: existing_html_summary(root)
            .unwrap_or_else(|| "target/xtal/cert/summary.html".to_string()),
        signature: string_field(&certificate, &["signature", "ed25519"])
            .unwrap_or_else(|| "unsigned-local-preview".to_string()),
    }
}

fn existing_html_summary(root: &Utf8Path) -> Option<String> {
    [
        "target/xtal/cert/summary.html",
        "target/cert/summary.html",
        "target/cert/certificate.html",
    ]
    .into_iter()
    .find(|path| root.join(path).exists())
    .map(str::to_string)
}

fn latest_op_json(session: &SessionSnapshot, prefix: &str) -> Option<Value> {
    session
        .op_log
        .iter()
        .rev()
        .find(|op| op.op.starts_with(prefix) && op.report_json.is_some())
        .and_then(|op| op.report_json.clone())
}

fn latest_trust_profile_arg(session: &SessionSnapshot) -> Option<String> {
    session
        .op_log
        .iter()
        .rev()
        .find(|op| op.op == "trust.certify.profile")
        .and_then(|op| {
            op.command.windows(2).find_map(|items| {
                if items[0] == "--profile" {
                    Some(items[1].clone())
                } else {
                    None
                }
            })
        })
}

fn read_json(path: &Utf8Path) -> Option<Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    find_key(value, keys)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn find_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key) {
                    return Some(value);
                }
            }
            map.values().find_map(|value| find_key(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| find_key(value, keys)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::summary;
    use loom_types::artifacts::{OpRecord, OperationStatus, TaskType};
    use loom_types::session::SessionSnapshot;
    use uuid::Uuid;

    #[test]
    fn certificate_summary_reads_certificate_json() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-cert-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("target/cert")).expect("mkdir");
        std::fs::write(
            root.join("target/cert/certificate.json"),
            r#"{"profile":"verified_core_pure_v1","operational_entry":"sort","signature":"abc"}"#,
        )
        .expect("cert");
        let session_id = Uuid::new_v4();
        let session =
            SessionSnapshot::new(session_id, "demo", root.to_string(), TaskType::NewBehavior);

        let cert = summary(root.as_path(), &session);

        assert_eq!(cert.profile, "verified_core_pure_v1");
        assert_eq!(cert.operational_entry, "sort");
        assert_eq!(cert.signature, "abc");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn certificate_summary_uses_latest_trust_profile_command() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-cert-profile-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("mkdir");
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", root.to_string(), TaskType::NewBehavior);
        session.op_log.push(OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id,
            op: "trust.certify.profile".to_string(),
            backend: "test".to_string(),
            command: vec![
                "x07".to_string(),
                "trust".to_string(),
                "certify".to_string(),
                "--profile".to_string(),
                "arch/trust/profiles/verified_core_pure_v1.json".to_string(),
            ],
            started_at: "1".to_string(),
            finished_at: Some("1".to_string()),
            status: OperationStatus::Succeeded,
            exit_code: Some(0),
            artifacts: vec!["target/cert/certificate.json".to_string()],
            notes: None,
            stdout: None,
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json: None,
            report_path: None,
        });

        let cert = summary(root.as_path(), &session);

        assert_eq!(
            cert.profile,
            "arch/trust/profiles/verified_core_pure_v1.json"
        );
        std::fs::remove_dir_all(root).ok();
    }
}
