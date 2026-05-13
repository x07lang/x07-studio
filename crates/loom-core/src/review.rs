use loom_adapters::command_runner::now_string;
use loom_types::api::{ProofCitation, ReviewConcern, ReviewRound};
use loom_types::artifacts::{OperationStatus, PlainEnglishSummary};
use loom_types::session::SessionSnapshot;

pub fn baseline_review(session: &SessionSnapshot, reviewer_id: &str, round: u32) -> ReviewRound {
    let mut concerns = Vec::new();

    if !session
        .op_log
        .iter()
        .any(|op| op.op.starts_with("tests.") && op.status == OperationStatus::Succeeded)
    {
        concerns.push(concern(
            "missing_test",
            "No generated test step is recorded for this realization.",
        ));
    }

    match latest_verify_status(session) {
        Some(OperationStatus::Succeeded) => {}
        Some(OperationStatus::Running | OperationStatus::Pending) => concerns.push(concern(
            "verify_pending",
            "Verification has not finished for the current implementation.",
        )),
        Some(OperationStatus::Failed) => concerns.push(concern(
            "boundary_violation",
            "The latest x07 verify step failed and must be repaired before accept.",
        )),
        None => concerns.push(concern(
            "missing_test",
            "No x07 verify evidence is recorded for the current implementation.",
        )),
    }

    if let Some(summary) = latest_summary(session) {
        if summary.scaffold_only {
            concerns.push(concern(
                "spec_drift",
                "The verified summary still points at a scaffold-only implementation.",
            ));
        }
    }

    let verdict = if concerns
        .iter()
        .any(|item| item.kind == "boundary_violation")
    {
        "block"
    } else if concerns.is_empty() {
        "accept"
    } else {
        "revise"
    };

    ReviewRound {
        schema_version: "x07.studio.review_round@0.1.0".to_string(),
        session_id: session.session_id,
        round,
        reviewer: reviewer_id.to_string(),
        verdict: verdict.to_string(),
        concerns,
        created_at: now_string(),
    }
}

fn latest_verify_status(session: &SessionSnapshot) -> Option<OperationStatus> {
    session
        .op_log
        .iter()
        .rev()
        .find(|op| op.op.starts_with("xtal.verify"))
        .map(|op| op.status.clone())
}

fn latest_summary(session: &SessionSnapshot) -> Option<PlainEnglishSummary> {
    session
        .op_log
        .iter()
        .rev()
        .find(|op| op.op == "summary.plain_english")
        .and_then(|op| op.report_json.clone())
        .and_then(|value| serde_json::from_value(value).ok())
}

fn concern(kind: &str, message: &str) -> ReviewConcern {
    ReviewConcern {
        kind: kind.to_string(),
        message: message.to_string(),
        citation: Some(ProofCitation {
            clause_id: kind.to_string(),
            proof_report: None,
            summary: message.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::baseline_review;
    use loom_types::artifacts::{OpRecord, OperationStatus, TaskType};
    use loom_types::session::SessionSnapshot;
    use uuid::Uuid;

    fn op(session_id: Uuid, name: &str, status: OperationStatus) -> OpRecord {
        OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id,
            op: name.to_string(),
            backend: "test".to_string(),
            command: Vec::new(),
            started_at: "1".to_string(),
            finished_at: Some("2".to_string()),
            status,
            exit_code: Some(0),
            artifacts: Vec::new(),
            notes: None,
            stdout: None,
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json: None,
            report_path: None,
        }
    }

    #[test]
    fn accepts_when_tests_and_verify_are_present() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.op_log.push(op(
            session.session_id,
            "tests.gen.write",
            OperationStatus::Succeeded,
        ));
        session.op_log.push(op(
            session.session_id,
            "xtal.verify",
            OperationStatus::Succeeded,
        ));

        let review = baseline_review(&session, "claude-code", 1);

        assert_eq!(review.verdict, "accept");
    }

    #[test]
    fn blocks_failed_verify() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.op_log.push(op(
            session.session_id,
            "xtal.verify",
            OperationStatus::Failed,
        ));

        let review = baseline_review(&session, "claude-code", 1);

        assert_eq!(review.verdict, "block");
    }
}
