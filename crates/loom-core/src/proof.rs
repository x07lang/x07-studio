use camino::Utf8Path;
use loom_types::api::{ProofEvidence, ProofEvidenceCitation, ProofObligation};
use loom_types::artifacts::{OperationStatus, PlainEnglishSummary};
use loom_types::session::SessionSnapshot;
use serde_json::Value;

pub fn for_behavior(
    root: &Utf8Path,
    session: &SessionSnapshot,
    behavior_id: &str,
) -> ProofEvidence {
    let summary = latest_summary(session);
    let behavior_text = summary
        .as_ref()
        .and_then(|summary| behavior_text_for_id(summary, behavior_id))
        .unwrap_or_else(|| behavior_id.replace('-', " "));
    let verify_json = latest_verify_json(root, session).unwrap_or(Value::Null);
    let verified = session
        .op_log
        .iter()
        .rev()
        .any(|op| op.op == "xtal.verify" && op.status == OperationStatus::Succeeded);
    let proved = verified && has_proof_signal(&verify_json);
    let status = if proved {
        "proved"
    } else if verified {
        "test-evidence"
    } else {
        "assumed"
    };
    let citations = proof_citations(root, session, &verify_json, status);
    let obligations = proof_obligations(&verify_json, behavior_id, &behavior_text, status);
    let z3_ms = first_u64(&verify_json, &["z3_ms", "solver_ms", "prove_ms"]);
    let assumptions = session
        .intent
        .as_ref()
        .map(|intent| intent.assumptions.clone())
        .unwrap_or_default();
    ProofEvidence {
        schema_version: "x07.studio.proof_evidence@0.1.0".to_string(),
        session_id: session.session_id,
        behavior_id: behavior_id.to_string(),
        status: status.to_string(),
        citations,
        obligations,
        z3_ms,
        assumptions,
    }
}

fn latest_summary(session: &SessionSnapshot) -> Option<PlainEnglishSummary> {
    session.op_log.iter().rev().find_map(|op| {
        if op.op == "summary.plain_english" {
            op.report_json
                .as_ref()
                .and_then(|json| serde_json::from_value(json.clone()).ok())
        } else {
            None
        }
    })
}

fn behavior_text_for_id(summary: &PlainEnglishSummary, behavior_id: &str) -> Option<String> {
    summary
        .behavior_promise_ids
        .iter()
        .position(|id| id == behavior_id)
        .and_then(|idx| summary.behavior_promises.get(idx).cloned())
        .or_else(|| {
            summary
                .behavior_promises
                .iter()
                .find(|text| crate::summarize::behavior_promise_id(text) == behavior_id)
                .cloned()
        })
}

fn latest_verify_json(root: &Utf8Path, session: &SessionSnapshot) -> Option<Value> {
    session
        .op_log
        .iter()
        .rev()
        .find(|op| op.op == "xtal.verify" && op.report_json.is_some())
        .and_then(|op| op.report_json.clone())
        .or_else(|| read_json(root.join("target/xtal/verify/summary.json").as_path()))
}

fn proof_citations(
    root: &Utf8Path,
    session: &SessionSnapshot,
    verify_json: &Value,
    status: &str,
) -> Vec<ProofEvidenceCitation> {
    let mut citations = Vec::new();
    if root.join("target/xtal/verify/summary.json").exists()
        || session.op_log.iter().any(|op| {
            op.artifacts
                .iter()
                .any(|artifact| artifact.contains("target/xtal/verify"))
        })
    {
        citations.push(ProofEvidenceCitation {
            kind: status.to_string(),
            file: "target/xtal/verify/summary.json".to_string(),
            region: Some("summary".to_string()),
        });
    }
    if let Some(path) = first_string(verify_json, &["proof_report", "proof_path"]) {
        citations.push(ProofEvidenceCitation {
            kind: "proof".to_string(),
            file: path,
            region: None,
        });
    }
    if citations.is_empty() {
        citations.push(ProofEvidenceCitation {
            kind: "session".to_string(),
            file: ".x07/studio/sessions".to_string(),
            region: Some("op_log".to_string()),
        });
    }
    citations
}

fn proof_obligations(
    verify_json: &Value,
    behavior_id: &str,
    behavior_text: &str,
    status: &str,
) -> Vec<ProofObligation> {
    if let Some(items) =
        find_key(verify_json, &["obligations", "proof_obligations"]).and_then(Value::as_array)
    {
        let obligations = items
            .iter()
            .enumerate()
            .map(|(idx, item)| ProofObligation {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{behavior_id}-{idx}")),
                goal: item
                    .get("goal")
                    .or_else(|| item.get("summary"))
                    .and_then(Value::as_str)
                    .unwrap_or(behavior_text)
                    .to_string(),
                status: item
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or(status)
                    .to_string(),
                note: item
                    .get("note")
                    .or_else(|| item.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
            .collect::<Vec<_>>();
        if !obligations.is_empty() {
            return obligations;
        }
    }
    vec![ProofObligation {
        id: behavior_id.to_string(),
        goal: behavior_text.to_string(),
        status: status.to_string(),
        note: Some(
            match status {
                "proved" => "Backed by the latest prove-capable verify report.",
                "test-evidence" => "No proof object was found; Studio is showing test evidence.",
                _ => "No current verify evidence was found for this behavior.",
            }
            .to_string(),
        ),
    }]
}

fn has_proof_signal(value: &Value) -> bool {
    first_u64(value, &["proof_count", "proved_count"]).unwrap_or(0) > 0
        || find_key(value, &["proofs", "obligations", "proof_obligations"])
            .and_then(Value::as_array)
            .map(|items| !items.is_empty())
            .unwrap_or(false)
}

fn read_json(path: &Utf8Path) -> Option<Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    find_key(value, keys)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn first_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    find_key(value, keys).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_f64().map(|num| num.max(0.0) as u64))
    })
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
    use super::for_behavior;
    use loom_types::artifacts::{OpRecord, OperationStatus, TaskType};
    use loom_types::session::SessionSnapshot;
    use uuid::Uuid;

    #[test]
    fn proof_evidence_falls_back_to_test_evidence() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-proof-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("target/xtal/verify")).expect("mkdir");
        std::fs::write(root.join("target/xtal/verify/summary.json"), "{}").expect("verify");
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", root.to_string(), TaskType::NewBehavior);
        session
            .op_log
            .push(op(session_id, "xtal.verify", OperationStatus::Succeeded));

        let evidence = for_behavior(root.as_path(), &session, "sorts-inputs");

        assert_eq!(evidence.status, "test-evidence");
        assert!(!evidence.citations.is_empty());
        std::fs::remove_dir_all(root).ok();
    }

    fn op(session_id: Uuid, name: &str, status: OperationStatus) -> OpRecord {
        OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id,
            op: name.to_string(),
            backend: "test".to_string(),
            command: Vec::new(),
            started_at: "1".to_string(),
            finished_at: Some("1".to_string()),
            status,
            exit_code: Some(0),
            artifacts: vec!["target/xtal/verify/summary.json".to_string()],
            notes: None,
            stdout: None,
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json: None,
            report_path: None,
        }
    }
}
