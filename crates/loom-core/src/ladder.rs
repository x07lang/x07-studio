use camino::Utf8Path;

use loom_types::api::{LadderRung, LadderState};
use loom_types::artifacts::OperationStatus;
use loom_types::session::SessionSnapshot;

pub fn ladder_state(root: &Utf8Path, session: &SessionSnapshot) -> LadderState {
    let rungs = rung_specs()
        .into_iter()
        .map(|spec| {
            let mut missing = Vec::new();
            let mut evidence = Vec::new();
            for path in spec.required_paths {
                if root.join(path).exists() || session_has_artifact(session, path) {
                    evidence.push(path.to_string());
                } else {
                    missing.push(path.to_string());
                }
            }
            if spec.id == "local_preview" && verified(session) {
                evidence.push("xtal.verify succeeded".to_string());
            } else if spec.id == "local_preview" && !verified(session) {
                missing.push("xtal.verify succeeded".to_string());
            }
            LadderRung {
                id: spec.id.to_string(),
                label: spec.label.to_string(),
                profile_path: spec.profile_path.map(str::to_string),
                satisfied: missing.is_empty(),
                missing,
                evidence,
            }
        })
        .collect::<Vec<_>>();
    let current_rung = rungs
        .iter()
        .rev()
        .find(|rung| rung.satisfied)
        .map(|rung| rung.id.clone())
        .unwrap_or_else(|| "local_preview".to_string());
    LadderState {
        current_rung,
        rungs,
    }
}

pub fn rung_profile_path(rung_id: &str) -> Option<&'static str> {
    rung_specs()
        .into_iter()
        .find(|spec| spec.id == rung_id)
        .and_then(|spec| spec.profile_path)
}

struct RungSpec {
    id: &'static str,
    label: &'static str,
    profile_path: Option<&'static str>,
    required_paths: &'static [&'static str],
}

fn rung_specs() -> Vec<RungSpec> {
    vec![
        RungSpec {
            id: "local_preview",
            label: "Local preview",
            profile_path: None,
            required_paths: &["x07.json", "target/xtal/verify/summary.json"],
        },
        RungSpec {
            id: "shareable",
            label: "Shareable",
            profile_path: Some("arch/trust/profiles/trusted_program_sandboxed_local_v1.json"),
            required_paths: &[
                "target/xtal/verify/summary.json",
                "arch/trust/profiles/trusted_program_sandboxed_local_v1.json",
            ],
        },
        RungSpec {
            id: "team",
            label: "Team",
            profile_path: Some("arch/trust/profiles/verified_core_pure_v1.json"),
            required_paths: &[
                "target/xtal/verify/summary.json",
                "arch/trust/profiles/verified_core_pure_v1.json",
            ],
        },
        RungSpec {
            id: "production",
            label: "Production",
            profile_path: Some("arch/trust/profiles/certified_capsule_v1.json"),
            required_paths: &[
                "target/xtal/verify/summary.json",
                "arch/trust/profiles/certified_capsule_v1.json",
            ],
        },
    ]
}

fn session_has_artifact(session: &SessionSnapshot, path: &str) -> bool {
    session
        .op_log
        .iter()
        .any(|op| op.artifacts.iter().any(|artifact| artifact == path))
}

fn verified(session: &SessionSnapshot) -> bool {
    session
        .op_log
        .iter()
        .any(|op| op.op == "xtal.verify" && op.status == OperationStatus::Succeeded)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use loom_types::artifacts::{OpRecord, OperationStatus, TaskType};
    use loom_types::session::SessionSnapshot;

    use super::ladder_state;

    #[test]
    fn local_preview_is_satisfied_after_verify_artifact() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8 temp")
            .join(format!("x07-studio-ladder-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("target/xtal/verify")).expect("mkdir");
        std::fs::write(root.join("x07.json"), "{}").expect("x07");
        std::fs::write(root.join("target/xtal/verify/summary.json"), "{}").expect("verify");
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", root.to_string(), TaskType::NewBehavior);
        session.op_log.push(OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id,
            op: "xtal.verify".to_string(),
            backend: "test".to_string(),
            command: Vec::new(),
            started_at: "1".to_string(),
            finished_at: Some("1".to_string()),
            status: OperationStatus::Succeeded,
            exit_code: Some(0),
            artifacts: vec!["target/xtal/verify/summary.json".to_string()],
            notes: None,
            stdout: None,
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json: None,
            report_path: None,
        });

        let state = ladder_state(root.as_path(), &session);

        assert_eq!(state.current_rung, "local_preview");
        assert!(state.rungs[0].satisfied);
        std::fs::remove_dir_all(root).ok();
    }
}
