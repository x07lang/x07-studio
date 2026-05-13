use loom_core::process_lane;
use loom_types::api::{AgentRole, StepStatus};
use loom_types::artifacts::{OpRecord, OperationStatus, TaskType};
use loom_types::session::SessionSnapshot;
use uuid::Uuid;

fn op(session_id: Uuid, name: &str) -> OpRecord {
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: name.to_string(),
        backend: "test".to_string(),
        command: Vec::new(),
        started_at: "100".to_string(),
        finished_at: Some("101".to_string()),
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
    }
}

#[test]
fn full_autopilot_run_projects_stable_process_lane() {
    let mut session = SessionSnapshot::new(
        Uuid::new_v4(),
        "process lane",
        "/tmp/process-lane",
        TaskType::NewBehavior,
    );
    for name in [
        "intent.formalize",
        "agent.handoff.claude-code",
        "spec.scaffold",
        "tests.gen.write",
        "impl.sync.write",
        "agent.realize.openai-codex",
        "xtal.verify",
        "review.round",
        "lint.report",
    ] {
        session.op_log.push(op(session.session_id, name));
    }

    let lane = process_lane::project(&session);
    let ids = lane
        .steps
        .iter()
        .take(10)
        .map(|step| step.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "intent", "agent_md", "clarify", "spec", "tests", "impl", "verify", "prove", "lint",
            "review"
        ]
    );
    assert_eq!(
        lane.steps
            .iter()
            .find(|step| step.id == "impl")
            .unwrap()
            .actor,
        AgentRole::Coder
    );
    assert_eq!(
        lane.steps
            .iter()
            .find(|step| step.id == "review")
            .unwrap()
            .status,
        StepStatus::Done
    );
}
