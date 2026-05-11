use thiserror::Error;

use loom_types::ops::SessionEvent;
use loom_types::session::{
    AllowedVerb, GlobalDoctrine, ProjectDoctrine, Room, SessionContract, SessionPhase,
    SessionSnapshot, TaskDoctrine, WritePolicy,
};

#[derive(Debug, Error)]
pub enum TransitionError {
    #[error("unknown session `{session_id}`")]
    UnknownSession { session_id: uuid::Uuid },
    #[error("illegal event `{event}` in phase `{phase:?}`")]
    IllegalTransition {
        phase: SessionPhase,
        event: &'static str,
    },
}

pub fn apply_event(
    session: &mut SessionSnapshot,
    event: SessionEvent,
) -> Result<(), TransitionError> {
    match event {
        SessionEvent::FormalizeIntent(intent) => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::IntentDrafting, SessionPhase::IntentReady],
                "formalize_intent",
            )?;
            session.intent = Some(*intent);
            session.phase = SessionPhase::IntentReady;
            session.room = Room::Intent;
        }
        SessionEvent::DraftSpec => {
            ensure_phase(&session.phase, &[SessionPhase::IntentReady], "draft_spec")?;
            session.phase = SessionPhase::SpecDraft;
            session.room = Room::Spec;
        }
        SessionEvent::ApproveSpec => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::SpecDraft, SessionPhase::SpecReview],
                "approve_spec",
            )?;
            session.phase = SessionPhase::SpecApproved;
            session.room = Room::Realization;
            session.contract = Some(SessionContract {
                schema_version: "x07.studio.session_contract@0.1.0".to_string(),
                session_id: session.session_id,
                workspace_root: session.root.clone(),
                global_doctrine: GlobalDoctrine {
                    mcp_tools: canonical_mcp_tools(),
                    doc_refs: canonical_doc_refs(),
                },
                project_doctrine: ProjectDoctrine {
                    xtal_manifest: "arch/xtal/xtal.json".to_string(),
                    agent_md: "AGENT.md".to_string(),
                    write_policy: WritePolicy {
                        agent_write_specs: false,
                        agent_write_arch: false,
                        paths: vec!["src/".to_string(), "tests/".to_string()],
                    },
                },
                task_doctrine: TaskDoctrine {
                    intent_ref: Some(format!(".x07/studio/sessions/{}.json", session.session_id)),
                    focus_paths: vec!["spec/".to_string(), "src/".to_string()],
                    baseline_refs: vec!["target/xtal/verify/summary.json".to_string()],
                },
                allowed_verbs: allowed_verbs_for_phase(&SessionPhase::SpecApproved),
            });
        }
        SessionEvent::ProposeRealization => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::SpecApproved],
                "propose_realization",
            )?;
            session.phase = SessionPhase::RealizationProposed;
            session.room = Room::Realization;
        }
        SessionEvent::AcceptRealization => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::RealizationProposed],
                "accept_realization",
            )?;
            session.phase = SessionPhase::VerifyRunning;
            session.room = Room::Verify;
        }
        SessionEvent::VerificationPassed => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::VerifyRunning],
                "verification_passed",
            )?;
            session.phase = SessionPhase::TrustReview;
            session.room = Room::Trust;
        }
        SessionEvent::VerificationFailed => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::VerifyRunning],
                "verification_failed",
            )?;
            session.phase = SessionPhase::RepairEligible;
            session.room = Room::Repair;
        }
        SessionEvent::RepairSpecPreserving => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::RepairEligible],
                "repair_spec_preserving",
            )?;
            session.phase = SessionPhase::VerifyRunning;
            session.room = Room::Verify;
        }
        SessionEvent::RepairSpecChanging => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::RepairEligible],
                "repair_spec_changing",
            )?;
            session.phase = SessionPhase::SpecReview;
            session.room = Room::Spec;
        }
        SessionEvent::ApproveTrust => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::TrustReview],
                "approve_trust",
            )?;
            session.phase = SessionPhase::CertifyRunning;
            session.room = Room::Trust;
        }
        SessionEvent::CertificationPassed => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::CertifyRunning],
                "certification_passed",
            )?;
            session.phase = SessionPhase::Certified;
            session.room = Room::Ops;
        }
        SessionEvent::IngestIncident => {
            ensure_phase(
                &session.phase,
                &[SessionPhase::Certified, SessionPhase::IncidentIngesting],
                "ingest_incident",
            )?;
            session.phase = SessionPhase::IncidentIngesting;
            session.room = Room::Ops;
        }
        SessionEvent::MoveRoom(room) => {
            session.room = room;
        }
        SessionEvent::AppendOp(op) => {
            session.op_log.push(*op);
        }
        SessionEvent::UpdateOp(op) => {
            if let Some(existing) = session.op_log.iter_mut().find(|item| item.id == op.id) {
                *existing = *op;
            } else {
                session.op_log.push(*op);
            }
        }
    }

    session.allowed_verbs = allowed_verbs_for_phase(&session.phase);
    Ok(())
}

fn ensure_phase(
    phase: &SessionPhase,
    allowed: &[SessionPhase],
    event: &'static str,
) -> Result<(), TransitionError> {
    if allowed.iter().any(|candidate| candidate == phase) {
        Ok(())
    } else {
        Err(TransitionError::IllegalTransition {
            phase: phase.clone(),
            event,
        })
    }
}

pub fn allowed_verbs_for_phase(phase: &SessionPhase) -> Vec<AllowedVerb> {
    match phase {
        SessionPhase::IntentDrafting => vec![AllowedVerb::IntentFormalize],
        SessionPhase::IntentReady => {
            vec![
                AllowedVerb::IntentFormalize,
                AllowedVerb::IntentReview,
                AllowedVerb::SpecEdit,
            ]
        }
        SessionPhase::SpecDraft => {
            vec![
                AllowedVerb::SpecEdit,
                AllowedVerb::SpecCheck,
                AllowedVerb::SpecApprove,
            ]
        }
        SessionPhase::SpecReview => {
            vec![
                AllowedVerb::SpecEdit,
                AllowedVerb::SpecCheck,
                AllowedVerb::SpecApprove,
            ]
        }
        SessionPhase::SpecApproved => vec![AllowedVerb::ImplSync],
        SessionPhase::RealizationProposed => vec![AllowedVerb::ImplReview, AllowedVerb::VerifyRun],
        SessionPhase::VerifyRunning => vec![AllowedVerb::VerifyRun],
        SessionPhase::RepairEligible => {
            vec![AllowedVerb::RepairRun, AllowedVerb::RepairSuggestSpecPatch]
        }
        SessionPhase::TrustReview => vec![AllowedVerb::TrustReview, AllowedVerb::CertifyRun],
        SessionPhase::CertifyRunning => vec![AllowedVerb::CertifyRun],
        SessionPhase::Certified => vec![AllowedVerb::IncidentIngest, AllowedVerb::ImproveRun],
        SessionPhase::IncidentIngesting => {
            vec![AllowedVerb::IncidentIngest, AllowedVerb::ImproveRun]
        }
        SessionPhase::HumanInterventionRequired => vec![AllowedVerb::IntentReview],
    }
}

fn canonical_mcp_tools() -> Vec<String> {
    [
        "x07.search_v1",
        "x07.doc_v1",
        "x07.context_pack_v1",
        "x07.exec_v1",
        "x07.patch_apply_v1",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn canonical_doc_refs() -> Vec<String> {
    [
        "x07/docs/getting-started/agent-quickstart.md",
        "x07/docs/getting-started/available-skills.md",
        "x07/docs/guides",
        "x07/docs/examples",
        "x07/docs/trust",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use loom_types::artifacts::{IntentPacket, TaskType};
    use loom_types::ops::SessionEvent;
    use loom_types::session::{AllowedVerb, Room, SessionPhase, SessionSnapshot};

    use super::{apply_event, TransitionError};

    #[test]
    fn lifecycle_happy_path_moves_through_xtal_rooms() {
        let session_id = Uuid::new_v4();
        let mut session = SessionSnapshot::new(
            session_id,
            "stable sorter",
            "/workspace",
            TaskType::NewBehavior,
        );

        apply_event(
            &mut session,
            SessionEvent::FormalizeIntent(Box::new(IntentPacket::demo(session_id, "/workspace"))),
        )
        .expect("intent formalization");
        assert_eq!(session.phase, SessionPhase::IntentReady);
        assert_eq!(session.room, Room::Intent);
        assert!(session.allowed_verbs.contains(&AllowedVerb::SpecEdit));

        apply_event(&mut session, SessionEvent::DraftSpec).expect("spec draft");
        assert_eq!(session.phase, SessionPhase::SpecDraft);
        assert_eq!(session.room, Room::Spec);

        apply_event(&mut session, SessionEvent::ApproveSpec).expect("spec approval");
        assert_eq!(session.phase, SessionPhase::SpecApproved);
        assert_eq!(session.room, Room::Realization);
        let contract = session.contract.as_ref().expect("session contract");
        assert_eq!(contract.workspace_root, "/workspace");
        assert!(!contract.project_doctrine.write_policy.agent_write_specs);
        assert_eq!(contract.allowed_verbs, vec![AllowedVerb::ImplSync]);
        assert!(contract
            .global_doctrine
            .doc_refs
            .iter()
            .any(|item| item.ends_with("agent-quickstart.md")));
        assert!(contract
            .global_doctrine
            .mcp_tools
            .contains(&"x07.doc_v1".to_string()));

        apply_event(&mut session, SessionEvent::ProposeRealization).expect("realization proposal");
        assert_eq!(session.phase, SessionPhase::RealizationProposed);

        apply_event(&mut session, SessionEvent::AcceptRealization).expect("realization accept");
        assert_eq!(session.phase, SessionPhase::VerifyRunning);
        assert_eq!(session.room, Room::Verify);

        apply_event(&mut session, SessionEvent::VerificationFailed).expect("verify failure");
        assert_eq!(session.phase, SessionPhase::RepairEligible);
        assert_eq!(session.room, Room::Repair);

        apply_event(&mut session, SessionEvent::RepairSpecChanging).expect("spec-changing repair");
        assert_eq!(session.phase, SessionPhase::SpecReview);
        assert_eq!(session.room, Room::Spec);
    }

    #[test]
    fn illegal_transition_keeps_session_state_unchanged() {
        let session_id = Uuid::new_v4();
        let mut session = SessionSnapshot::new(
            session_id,
            "stable sorter",
            "/workspace",
            TaskType::NewBehavior,
        );

        let error = apply_event(&mut session, SessionEvent::DraftSpec).expect_err("must fail");
        assert!(matches!(
            error,
            TransitionError::IllegalTransition {
                phase: SessionPhase::IntentDrafting,
                event: "draft_spec",
            }
        ));
        assert_eq!(session.phase, SessionPhase::IntentDrafting);
        assert_eq!(session.room, Room::Intent);
    }
}
