use loom_adapters::command_runner::now_string;
use loom_types::api::{AutopilotDecision, AutopilotPolicy, TurnQuestion};
use loom_types::artifacts::{OperationStatus, WitnessKind};
use loom_types::session::{SessionPhase, SessionSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutopilotAction {
    AutoClarify,
    AutoAnswer,
    AutoApproveSpec,
    AutoBuild,
    AutoRealize,
    AutoClimb,
    Pause,
}

#[derive(Debug, Clone)]
pub struct AutopilotPlan {
    pub action: AutopilotAction,
    pub decision: AutopilotDecision,
}

pub fn decide_next(state: &SessionSnapshot, policy: &AutopilotPolicy) -> AutopilotPlan {
    let (action, stage, reason) = if state.intent.is_none() {
        (
            AutopilotAction::Pause,
            "intent",
            "No intent has been formalized for this session.",
        )
    } else if let Some(op) = latest_budget_exhaustion(state) {
        (
            AutopilotAction::Pause,
            "budget_exhausted",
            op.notes
                .as_deref()
                .unwrap_or("A pipeline step exceeded its budget. The partial work is preserved; pick a continuation."),
        )
    } else if needs_initial_clarify(state) {
        (
            AutopilotAction::AutoClarify,
            "clarify",
            "Autopilot runs one supervised clarify pass before approving the spec.",
        )
    } else if unanswered_high_confidence_questions(state, policy) {
        (
            AutopilotAction::AutoAnswer,
            "clarify_answer",
            "Clarify questions have bounded defaults above the policy confidence threshold.",
        )
    } else if matches!(
        state.phase,
        SessionPhase::IntentReady | SessionPhase::SpecDraft | SessionPhase::SpecReview
    ) {
        (
            AutopilotAction::AutoApproveSpec,
            "spec_approve",
            "Intent is stable enough to draft and approve the initial spec.",
        )
    } else if matches!(
        state.phase,
        SessionPhase::SpecApproved
            | SessionPhase::RealizationProposed
            | SessionPhase::VerifyRunning
    ) || !verified(state)
    {
        (
            AutopilotAction::AutoBuild,
            "build_run",
            "The session has not produced verified evidence yet.",
        )
    } else if scaffold_summary(state) {
        if realize_progress_stalled(state) {
            (
                AutopilotAction::Pause,
                "realize_stalled",
                "Two consecutive realize attempts did not clear the scaffold-only flag; pausing for human review.",
            )
        } else {
            (
                AutopilotAction::AutoRealize,
                "realize",
                "Verified evidence still points at a scaffold-only implementation.",
            )
        }
    } else if policy.auto_climb_to.is_some() && !certified_or_climbed(state, policy) {
        (
            AutopilotAction::AutoClimb,
            "ladder_climb",
            "Policy allows climbing the shipping ladder after verification.",
        )
    } else {
        (
            AutopilotAction::Pause,
            "complete",
            "Autopilot reached the configured stopping point.",
        )
    };
    let action_label = if matches!(action, AutopilotAction::Pause) {
        "user"
    } else {
        "auto"
    };
    AutopilotPlan {
        action,
        decision: AutopilotDecision {
            at: now_string(),
            stage: stage.to_string(),
            action: action_label.to_string(),
            reason: reason.to_string(),
        },
    }
}

fn latest_budget_exhaustion(state: &SessionSnapshot) -> Option<&loom_types::artifacts::OpRecord> {
    state
        .op_log
        .iter()
        .rev()
        .find(|op| op.op == "pipeline.budget_exhausted")
}

pub fn score_clarify_question(question: &TurnQuestion) -> f32 {
    if question.answer.is_some() {
        return 1.0;
    }
    if question.options.len() >= 2 {
        return 0.9;
    }
    if question.witness_kind == WitnessKind::ForbiddenBehavior {
        return 0.75;
    }
    0.4
}

fn unanswered_high_confidence_questions(state: &SessionSnapshot, policy: &AutopilotPolicy) -> bool {
    let Some(intent) = &state.intent else {
        return false;
    };
    let unanswered = intent
        .clarification_history
        .iter()
        .filter(|turn| turn.answer_text.is_none())
        .map(|turn| TurnQuestion {
            id: turn.question_id.clone(),
            text: turn.question_text.clone(),
            witness_kind: turn.witness_kind.clone(),
            options: turn.options.clone(),
            answer: None,
        })
        .collect::<Vec<_>>();
    !unanswered.is_empty()
        && unanswered
            .iter()
            .all(|question| score_clarify_question(question) >= policy.auto_answer_min_confidence)
}

fn needs_initial_clarify(state: &SessionSnapshot) -> bool {
    matches!(state.phase, SessionPhase::IntentReady)
        && state
            .intent
            .as_ref()
            .is_some_and(|intent| intent.clarification_history.is_empty())
}

fn verified(state: &SessionSnapshot) -> bool {
    state
        .op_log
        .iter()
        .any(|op| op.op == "xtal.verify" && op.status == OperationStatus::Succeeded)
}

fn scaffold_summary(state: &SessionSnapshot) -> bool {
    state
        .op_log
        .iter()
        .rev()
        .find(|op| op.op == "summary.plain_english")
        .and_then(|op| op.report_json.as_ref())
        .and_then(|value| value.get("scaffold_only"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

const MAX_REALIZE_WITHOUT_PROGRESS: usize = 2;

fn realize_progress_stalled(state: &SessionSnapshot) -> bool {
    let mut consecutive_no_progress: usize = 0;
    let mut realize_since_last_summary = false;
    for op in &state.op_log {
        if op.op.starts_with("agent.realize.") && op.status == OperationStatus::Succeeded {
            realize_since_last_summary = true;
        } else if op.op == "summary.plain_english" && realize_since_last_summary {
            realize_since_last_summary = false;
            let scaffold = op
                .report_json
                .as_ref()
                .and_then(|value| value.get("scaffold_only"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if scaffold {
                consecutive_no_progress += 1;
            } else {
                consecutive_no_progress = 0;
            }
        }
    }
    consecutive_no_progress >= MAX_REALIZE_WITHOUT_PROGRESS
}

fn certified_or_climbed(state: &SessionSnapshot, policy: &AutopilotPolicy) -> bool {
    let Some(target) = policy.auto_climb_to.as_deref() else {
        return true;
    };
    state.phase == SessionPhase::Certified
        || state
            .op_log
            .iter()
            .any(|op| op.op == format!("autopilot.ladder.{target}"))
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{decide_next, score_clarify_question, AutopilotAction};
    use loom_types::api::{AutopilotPolicy, TurnQuestion};
    use loom_types::artifacts::{
        ClarificationTurn, IntentPacket, OperationStatus, TaskType, WitnessKind,
    };
    use loom_types::session::{SessionPhase, SessionSnapshot};

    #[test]
    fn option_question_scores_high_confidence() {
        let question = TurnQuestion {
            id: "q1".to_string(),
            text: "Empty input?".to_string(),
            witness_kind: WitnessKind::ForbiddenBehavior,
            options: vec!["Reject".to_string(), "Return empty".to_string()],
            answer: None,
        };

        assert!(score_clarify_question(&question) >= 0.7);
    }

    #[test]
    fn intent_ready_auto_approves_spec() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.phase = SessionPhase::IntentReady;
        let mut intent = IntentPacket::demo(session.session_id, "/tmp/demo");
        intent.clarification_history.push(ClarificationTurn {
            question_id: "q1".to_string(),
            question_text: "Empty input?".to_string(),
            witness_kind: WitnessKind::ForbiddenBehavior,
            round: 1,
            agent_id: "claude-code".to_string(),
            options: vec!["Reject".to_string(), "Return empty".to_string()],
            question_recorded_at: "1".to_string(),
            answer_text: Some("Reject".to_string()),
            answer_recorded_at: Some("2".to_string()),
        });
        session.intent = Some(intent);

        let plan = decide_next(&session, &AutopilotPolicy::default());

        assert_eq!(plan.action, AutopilotAction::AutoApproveSpec);
    }

    #[test]
    fn high_confidence_clarify_auto_answers() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.phase = SessionPhase::IntentReady;
        let mut intent = IntentPacket::demo(session.session_id, "/tmp/demo");
        intent.clarification_history.push(ClarificationTurn {
            question_id: "q1".to_string(),
            question_text: "Empty input?".to_string(),
            witness_kind: WitnessKind::ForbiddenBehavior,
            round: 1,
            agent_id: "claude-code".to_string(),
            options: vec!["Reject".to_string(), "Return empty".to_string()],
            question_recorded_at: "1".to_string(),
            answer_text: None,
            answer_recorded_at: None,
        });
        session.intent = Some(intent);

        let plan = decide_next(&session, &AutopilotPolicy::default());

        assert_eq!(plan.action, AutopilotAction::AutoAnswer);
    }

    #[test]
    fn fresh_intent_runs_clarify_before_spec_approval() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.phase = SessionPhase::IntentReady;
        session.intent = Some(IntentPacket::demo(session.session_id, "/tmp/demo"));
        let plan = decide_next(&session, &AutopilotPolicy::default());

        assert_eq!(plan.action, AutopilotAction::AutoClarify);
        assert_eq!(plan.decision.stage, "clarify");
    }

    fn op_record(
        session_id: Uuid,
        op: &str,
        report_json: Option<serde_json::Value>,
    ) -> loom_types::artifacts::OpRecord {
        loom_types::artifacts::OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id,
            op: op.to_string(),
            backend: "test".to_string(),
            command: Vec::new(),
            started_at: "1".to_string(),
            finished_at: Some("1".to_string()),
            status: OperationStatus::Succeeded,
            exit_code: Some(0),
            artifacts: Vec::new(),
            notes: None,
            stdout: None,
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json,
            report_path: None,
        }
    }

    #[test]
    fn cleared_scaffold_after_first_summary_does_not_loop() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.phase = SessionPhase::TrustReview;
        session.intent = Some(IntentPacket::demo(session.session_id, "/tmp/demo"));
        let sid = session.session_id;
        session.op_log.extend([
            op_record(sid, "xtal.verify", None),
            op_record(
                sid,
                "summary.plain_english",
                Some(serde_json::json!({ "scaffold_only": true })),
            ),
            op_record(sid, "agent.realize.claude-code", None),
            op_record(
                sid,
                "summary.plain_english",
                Some(serde_json::json!({ "scaffold_only": false })),
            ),
        ]);

        let plan = decide_next(&session, &AutopilotPolicy::default());

        // Most-recent summary cleared scaffold_only — autopilot must move on,
        // not keep firing realize because an earlier summary had scaffold_only=true.
        assert_ne!(plan.action, AutopilotAction::AutoRealize);
    }

    #[test]
    fn two_stalled_realize_attempts_pause_autopilot() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.phase = SessionPhase::TrustReview;
        session.intent = Some(IntentPacket::demo(session.session_id, "/tmp/demo"));
        let sid = session.session_id;
        let scaffold_report = serde_json::json!({ "scaffold_only": true });
        session.op_log.extend([
            op_record(sid, "xtal.verify", None),
            op_record(sid, "agent.realize.claude-code", None),
            op_record(sid, "summary.plain_english", Some(scaffold_report.clone())),
            op_record(sid, "agent.realize.claude-code", None),
            op_record(sid, "summary.plain_english", Some(scaffold_report)),
        ]);

        let plan = decide_next(&session, &AutopilotPolicy::default());

        assert_eq!(plan.action, AutopilotAction::Pause);
        assert_eq!(plan.decision.stage, "realize_stalled");
    }

    #[test]
    fn single_scaffold_summary_still_attempts_realize() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.phase = SessionPhase::TrustReview;
        session.intent = Some(IntentPacket::demo(session.session_id, "/tmp/demo"));
        let sid = session.session_id;
        session.op_log.extend([
            op_record(sid, "xtal.verify", None),
            op_record(
                sid,
                "summary.plain_english",
                Some(serde_json::json!({ "scaffold_only": true })),
            ),
        ]);

        let plan = decide_next(&session, &AutopilotPolicy::default());

        assert_eq!(plan.action, AutopilotAction::AutoRealize);
    }

    #[test]
    fn verified_session_pauses() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.phase = SessionPhase::TrustReview;
        session.intent = Some(IntentPacket::demo(session.session_id, "/tmp/demo"));
        session.op_log.push(loom_types::artifacts::OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id: session.session_id,
            op: "xtal.verify".to_string(),
            backend: "test".to_string(),
            command: Vec::new(),
            started_at: "1".to_string(),
            finished_at: Some("1".to_string()),
            status: OperationStatus::Succeeded,
            exit_code: Some(0),
            artifacts: Vec::new(),
            notes: None,
            stdout: None,
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json: None,
            report_path: None,
        });

        let plan = decide_next(&session, &AutopilotPolicy::default());

        assert_eq!(plan.action, AutopilotAction::Pause);
    }
}
