use loom_types::api::{
    AgentRole, AutopilotPolicy, CanonicalStep, ProcessLane, StepBudget, StepStatus,
};
use loom_types::artifacts::{OpRecord, OperationStatus};
use loom_types::session::SessionSnapshot;

const PROCESS_SCHEMA: &str = "x07.studio.process_lane@0.1.0";
const STEP_SCHEMA: &str = "x07.studio.canonical_step@0.1.0";

const STEP_ORDER: &[(&str, &str, AgentRole)] = &[
    ("intent", "Capture intent", AgentRole::Conductor),
    ("agent_md", "Sync AGENT.md", AgentRole::Architect),
    ("clarify", "Clarify assumptions", AgentRole::Architect),
    ("spec", "Draft and check spec", AgentRole::Architect),
    ("tests", "Generate tests", AgentRole::Conductor),
    ("impl", "Write implementation", AgentRole::Coder),
    ("verify", "Verify behavior", AgentRole::Conductor),
    ("prove", "Prove properties", AgentRole::Conductor),
    ("lint", "Run lint", AgentRole::Conductor),
    ("review", "Review implementation", AgentRole::Reviewer),
    ("repair", "Repair failures", AgentRole::Coder),
    ("arch_check", "Check architecture", AgentRole::Conductor),
    ("lockfile", "Check lockfile", AgentRole::Conductor),
    ("migrate", "Check migration", AgentRole::Conductor),
    ("pbt", "Run PBT", AgentRole::Conductor),
    ("ladder_climb", "Climb release rung", AgentRole::Conductor),
    ("certify", "Certify release", AgentRole::Conductor),
];

pub fn project(session: &SessionSnapshot) -> ProcessLane {
    let mut steps = STEP_ORDER
        .iter()
        .map(|(id, label, actor)| empty_step(id, label, *actor))
        .collect::<Vec<_>>();

    for op in &session.op_log {
        let Some(step_id) = step_id_for_op(&op.op) else {
            continue;
        };
        let Some(index) = steps.iter().position(|step| step.id == step_id) else {
            continue;
        };
        let step = &mut steps[index];
        step.status = status_for_op(op);
        step.actor = actor_for_op(op, step.actor);
        step.started_at = Some(op.started_at.clone());
        step.finished_at = op.finished_at.clone();
        step.elapsed_ms = elapsed_ms(op);
        step.op_id = Some(op.id);
        step.narration = narration_for_op(op, step);
        if op.op == "pipeline.budget_exhausted" {
            step.budget = budget_from_op(op);
        }
        if op.op == "review.round" {
            step.round = op
                .report_json
                .as_ref()
                .and_then(|value| value.get("round"))
                .and_then(|value| value.get("round"))
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as u32);
        }
    }

    let current_index = steps
        .iter()
        .position(|step| matches!(step.status, StepStatus::Running | StepStatus::Stalled))
        .or_else(|| {
            steps
                .iter()
                .position(|step| step.status == StepStatus::Pending)
        });
    let next_index = current_index.and_then(|index| {
        steps
            .iter()
            .enumerate()
            .skip(index + 1)
            .find(|(_, step)| step.status == StepStatus::Pending)
            .map(|(idx, _)| idx)
    });

    if let Some(next) = next_index {
        let actor = steps[next].actor;
        if let Some(current) = current_index.and_then(|index| steps.get_mut(index)) {
            current.next_actor = Some(actor);
        }
    }

    ProcessLane {
        schema_version: PROCESS_SCHEMA.to_string(),
        session_id: session.session_id,
        steps,
        current_index,
        next_index,
    }
}

pub fn forecast_next(lane: &ProcessLane, _policy: &AutopilotPolicy) -> Vec<CanonicalStep> {
    let start = lane.current_index.or(lane.next_index).unwrap_or_default();
    lane.steps
        .iter()
        .skip(start)
        .filter(|step| step.status == StepStatus::Pending)
        .take(3)
        .cloned()
        .collect()
}

fn empty_step(id: &str, label: &str, actor: AgentRole) -> CanonicalStep {
    CanonicalStep {
        schema_version: STEP_SCHEMA.to_string(),
        id: id.to_string(),
        label: label.to_string(),
        actor,
        status: StepStatus::Pending,
        started_at: None,
        finished_at: None,
        elapsed_ms: None,
        op_id: None,
        narration: forecast_narration(id, actor),
        next_actor: None,
        budget: None,
        round: None,
    }
}

fn step_id_for_op(op: &str) -> Option<&'static str> {
    if op.starts_with("intent.formalize") {
        Some("intent")
    } else if op.starts_with("agent.contract") || op.starts_with("agent.handoff") {
        Some("agent_md")
    } else if op.contains("clarify") {
        Some("clarify")
    } else if op.starts_with("spec.") {
        Some("spec")
    } else if op.starts_with("tests.") {
        Some("tests")
    } else if op.starts_with("impl.")
        || op.starts_with("agent.realize.")
        || op == "synthesis.template"
    {
        Some("impl")
    } else if op == "review.round" {
        Some("review")
    } else if op.starts_with("xtal.verify") {
        Some("verify")
    } else if op.contains("prove") {
        Some("prove")
    } else if op.starts_with("lint.") {
        Some("lint")
    } else if op.starts_with("xtal.repair") || op.starts_with("xtal.improve") {
        Some("repair")
    } else if op.starts_with("arch.check") {
        Some("arch_check")
    } else if op.contains("lock") {
        Some("lockfile")
    } else if op.contains("migrate") {
        Some("migrate")
    } else if op.contains("pbt") {
        Some("pbt")
    } else if op.contains("ladder") || op.contains("release") {
        Some("ladder_climb")
    } else if op.contains("certify") || op.contains("certificate") {
        Some("certify")
    } else if op == "pipeline.budget_exhausted" {
        Some("impl")
    } else {
        None
    }
}

fn status_for_op(op: &OpRecord) -> StepStatus {
    match op.status {
        OperationStatus::Pending => StepStatus::Pending,
        OperationStatus::Running => StepStatus::Running,
        OperationStatus::Succeeded => StepStatus::Done,
        OperationStatus::Failed => StepStatus::Stalled,
    }
}

fn actor_for_op(op: &OpRecord, fallback: AgentRole) -> AgentRole {
    if op.op == "review.round" {
        AgentRole::Reviewer
    } else if op.op.starts_with("agent.realize.") || op.op == "pipeline.budget_exhausted" {
        AgentRole::Coder
    } else if op.op.starts_with("agent.") || op.op.starts_with("spec.") {
        AgentRole::Architect
    } else if op.op.starts_with("xtal.repair") {
        AgentRole::Coder
    } else {
        fallback
    }
}

fn narration_for_op(op: &OpRecord, step: &CanonicalStep) -> String {
    match step.actor {
        AgentRole::Conductor => format!("Studio ran `{}`.", op.op),
        AgentRole::Architect => format!("Architect advanced `{}`.", op.op),
        AgentRole::Coder => format!("Coder worked on `{}`.", op.op),
        AgentRole::Reviewer => format!("Reviewer inspected `{}`.", op.op),
    }
}

fn forecast_narration(id: &str, actor: AgentRole) -> String {
    let who = match actor {
        AgentRole::Conductor => "Studio",
        AgentRole::Architect => "Claude",
        AgentRole::Coder => "Codex",
        AgentRole::Reviewer => "Reviewer",
    };
    format!("{who} is expected to handle `{id}`.")
}

fn elapsed_ms(op: &OpRecord) -> Option<u64> {
    let start = op.started_at.parse::<u64>().ok()?;
    let finish = op.finished_at.as_deref()?.parse::<u64>().ok()?;
    finish.checked_sub(start).map(|seconds| seconds * 1_000)
}

fn budget_from_op(op: &OpRecord) -> Option<StepBudget> {
    op.report_json
        .as_ref()
        .and_then(|value| value.get("budget"))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

#[cfg(test)]
mod tests {
    use super::project;
    use loom_types::api::StepStatus;
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
            started_at: "10".to_string(),
            finished_at: Some("12".to_string()),
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
    fn empty_session_projects_pending_steps() {
        let session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);

        let lane = project(&session);

        assert_eq!(lane.steps[0].id, "intent");
        assert_eq!(lane.steps[0].status, StepStatus::Pending);
        assert_eq!(lane.current_index, Some(0));
    }

    #[test]
    fn maps_ops_to_canonical_steps_and_elapsed_time() {
        let mut session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);
        session.op_log.push(op(
            session.session_id,
            "agent.realize.openai-codex",
            OperationStatus::Succeeded,
        ));

        let lane = project(&session);
        let impl_step = lane.steps.iter().find(|step| step.id == "impl").unwrap();

        assert_eq!(impl_step.status, StepStatus::Done);
        assert_eq!(impl_step.elapsed_ms, Some(2_000));
    }
}
