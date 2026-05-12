use serde_json::Value;

use loom_adapters::x07_cli::CliAdapter;
use loom_types::artifacts::{IntentPacket, IntentSource};

#[derive(Debug, Clone, PartialEq)]
pub struct GenpackHandoffContext {
    pub archetype: &'static str,
    pub schema: Option<Value>,
    pub grammar: Option<String>,
}

pub async fn handoff_context(
    cli: &CliAdapter,
    intent: &IntentPacket,
) -> Option<GenpackHandoffContext> {
    let archetype = detect_archetype_from_intent(intent)?;
    let (schema, grammar) = tokio::join!(
        fetch_archetype_schema(cli, archetype),
        fetch_archetype_grammar(cli, archetype)
    );
    Some(GenpackHandoffContext {
        archetype,
        schema,
        grammar,
    })
}

pub async fn fetch_archetype_schema(cli: &CliAdapter, archetype: &str) -> Option<Value> {
    cli.service_genpack_schema(archetype).await.ok()
}

pub async fn fetch_archetype_grammar(cli: &CliAdapter, archetype: &str) -> Option<String> {
    cli.service_genpack_grammar(archetype).await.ok()
}

pub fn detect_archetype_from_intent(intent: &IntentPacket) -> Option<&'static str> {
    let haystack = intent_haystack(intent);
    let candidates = [
        (
            "api-cell",
            &[
                "api gateway",
                "api cell",
                "api endpoint",
                "http api",
                "rest api",
                "gateway",
            ][..],
        ),
        (
            "event-consumer",
            &[
                "event consumer",
                "consume event",
                "consumer",
                "event stream",
                "queue event",
            ][..],
        ),
        (
            "scheduled-job",
            &[
                "scheduled job",
                "scheduler",
                "cron",
                "nightly job",
                "periodic job",
            ][..],
        ),
        (
            "policy-service",
            &[
                "policy service",
                "authorization policy",
                "access policy",
                "guardrail",
                "policy engine",
            ][..],
        ),
        (
            "workflow-service",
            &[
                "workflow service",
                "workflow",
                "state machine",
                "orchestration",
                "long-running process",
            ][..],
        ),
    ];
    candidates
        .iter()
        .find(|(_, needles)| needles.iter().any(|needle| haystack.contains(needle)))
        .map(|(archetype, _)| *archetype)
}

fn intent_haystack(intent: &IntentPacket) -> String {
    let mut parts = Vec::new();
    for target in &intent.targets {
        parts.push(target.module_id.as_str());
        if let Some(entry) = &target.entry {
            parts.push(entry);
        }
    }
    parts.extend(intent.examples.iter().map(String::as_str));
    parts.extend(intent.constraints.iter().map(String::as_str));
    parts.extend(intent.policy_implications.iter().map(String::as_str));
    parts.extend(intent.ambiguities.iter().map(String::as_str));
    parts.extend(intent.assumptions.iter().map(String::as_str));
    for witness in &intent.witnesses {
        parts.push(witness.text.as_str());
    }
    for turn in &intent.clarification_history {
        parts.push(turn.question_text.as_str());
        if let Some(answer) = &turn.answer_text {
            parts.push(answer);
        }
    }
    match &intent.source {
        IntentSource::Text { raw } | IntentSource::Spec { raw } => parts.push(raw),
        IntentSource::Voice { transcript } => parts.push(transcript),
        IntentSource::Incident { path }
        | IntentSource::Sketch { path }
        | IntentSource::Image { path, .. } => parts.push(path),
    }
    parts.join("\n").to_lowercase()
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use loom_types::artifacts::{IntentTarget, TaskType, Witness, WitnessKind};

    use super::*;

    fn intent(raw: &str) -> IntentPacket {
        IntentPacket {
            schema_version: "x07.studio.intent_packet@0.1.0".to_string(),
            session_id: Uuid::nil(),
            workspace_root: "/workspace".to_string(),
            task_type: TaskType::NewBehavior,
            targets: vec![IntentTarget {
                module_id: "svc.test".to_string(),
                entry: Some("run_v1".to_string()),
            }],
            examples: Vec::new(),
            constraints: Vec::new(),
            policy_implications: Vec::new(),
            ambiguities: Vec::new(),
            assumptions: Vec::new(),
            witnesses: vec![Witness {
                kind: WitnessKind::DesiredBehavior,
                text: raw.to_string(),
            }],
            source: IntentSource::Text {
                raw: raw.to_string(),
            },
            clarification_history: Vec::new(),
        }
    }

    #[test]
    fn detects_service_archetypes_from_intent_text() {
        assert_eq!(
            detect_archetype_from_intent(&intent("Build an API gateway for account reads.")),
            Some("api-cell")
        );
        assert_eq!(
            detect_archetype_from_intent(&intent("Create an event consumer for billing events.")),
            Some("event-consumer")
        );
        assert_eq!(
            detect_archetype_from_intent(&intent("Run a nightly scheduled job.")),
            Some("scheduled-job")
        );
        assert_eq!(
            detect_archetype_from_intent(&intent("Add an access policy service.")),
            Some("policy-service")
        );
        assert_eq!(
            detect_archetype_from_intent(&intent("Model a workflow service as a state machine.")),
            Some("workflow-service")
        );
    }

    #[test]
    fn ignores_non_service_intents() {
        assert_eq!(
            detect_archetype_from_intent(&intent("Build a byte-array sorter.")),
            None
        );
    }
}
