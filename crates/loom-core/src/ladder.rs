use camino::Utf8Path;

use loom_types::api::{LadderRung, LadderState, RungGate};
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
                } else if spec.profile_path == Some(path)
                    && session_certified_profile(session, path)
                {
                    evidence.push(format!("trust profile certified: {path}"));
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
                gates: rung_gates(root, session, spec.id, spec.profile_path, &missing),
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

fn rung_gates(
    root: &Utf8Path,
    session: &SessionSnapshot,
    rung_id: &str,
    profile_path: Option<&str>,
    missing: &[String],
) -> Vec<RungGate> {
    let mut gates = match rung_id {
        "local_preview" => vec![
            gate(
                "xtal-verify",
                "XTAL verify",
                "The current implementation passes the project verify lane.",
            ),
            gate(
                "solve-default",
                "Solve-world default",
                "The local preview starts from deterministic solve-world execution.",
            ),
        ],
        "shareable" => vec![
            gate(
                "sandbox-profile",
                "Sandbox profile",
                "A sandbox trust profile is present before the project is shared.",
            ),
            gate(
                "arch-check",
                "Architecture check",
                "Repo-level architecture invariants pass.",
            ),
            gate(
                "lockfile",
                "Lockfile check",
                "The package lockfile is present and checked before sharing.",
            ),
            gate(
                "review-diff",
                "Review diff gates",
                "Capability, world, proof, and budget changes are reviewable before sharing.",
            ),
        ],
        "team" => vec![
            gate(
                "verified-core",
                "Verified core",
                "Team handoff requires the verified-core trust profile.",
            ),
            gate(
                "proof-coverage",
                "Proof coverage",
                "Proof and generated-test evidence are available for team review.",
            ),
            gate(
                "arch-check",
                "Architecture check",
                "Repo-level architecture invariants pass before team handoff.",
            ),
        ],
        "production" => vec![
            gate(
                "certified-capsule",
                "Certified capsule",
                "A release capsule can be certified with bundled evidence.",
            ),
            gate(
                "provenance",
                "Provenance",
                "Production promotion records provenance and certificate artifacts.",
            ),
            gate(
                "arch-check",
                "Architecture check",
                "Repo-level architecture invariants pass before production promotion.",
            ),
        ],
        _ => vec![gate("rung", "Rung gate", "Studio tracks this trust rung.")],
    };
    for gate in &mut gates {
        let profile_missing = profile_path
            .map(|profile| missing.iter().any(|item| item == profile))
            .unwrap_or(false);
        let verify_missing = missing
            .iter()
            .any(|item| item.contains("xtal.verify") && gate.id.contains("verify"));
        gate.currently_satisfied = !verify_missing
            && match gate.id.as_str() {
                "sandbox-profile" | "verified-core" | "certified-capsule" => !profile_missing,
                "arch-check" => session_op_succeeded(session, "arch.check"),
                "lockfile" => root.join("x07.lock.json").is_file(),
                _ => true,
            };
    }
    gates
}

fn gate(id: &str, label: &str, description: &str) -> RungGate {
    RungGate {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        currently_satisfied: true,
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

fn session_certified_profile(session: &SessionSnapshot, profile_path: &str) -> bool {
    session.op_log.iter().any(|op| {
        op.op == "trust.certify.profile"
            && op.status == OperationStatus::Succeeded
            && op
                .command
                .windows(2)
                .any(|items| items[0] == "--profile" && items[1] == profile_path)
    })
}

fn verified(session: &SessionSnapshot) -> bool {
    session
        .op_log
        .iter()
        .any(|op| op.op == "xtal.verify" && op.status == OperationStatus::Succeeded)
}

fn session_op_succeeded(session: &SessionSnapshot, op_name: &str) -> bool {
    session
        .op_log
        .iter()
        .any(|op| op.op == op_name && op.status == OperationStatus::Succeeded)
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

    #[test]
    fn profile_certification_satisfies_matching_rung_gate() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8 temp")
            .join(format!("x07-studio-ladder-profile-{}", Uuid::new_v4()));
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
            started_at: "2".to_string(),
            finished_at: Some("2".to_string()),
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
        session.op_log.push(OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id,
            op: "arch.check".to_string(),
            backend: "test".to_string(),
            command: Vec::new(),
            started_at: "3".to_string(),
            finished_at: Some("3".to_string()),
            status: OperationStatus::Succeeded,
            exit_code: Some(0),
            artifacts: vec!["arch/".to_string()],
            notes: None,
            stdout: None,
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json: None,
            report_path: None,
        });

        let state = ladder_state(root.as_path(), &session);

        assert_eq!(state.current_rung, "team");
        let team = state
            .rungs
            .iter()
            .find(|rung| rung.id == "team")
            .expect("team rung");
        assert!(team.satisfied);
        assert!(team.gates.iter().all(|gate| gate.currently_satisfied));
        std::fs::remove_dir_all(root).ok();
    }
}
