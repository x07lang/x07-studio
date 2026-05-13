use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use loom_types::api::{
    AgentStreamEvent, RealizeQuorumRound, SessionTurn, TurnEvidence, TurnQuestion,
};
use loom_types::artifacts::{IntentPacket, IntentSource, OpRecord, PlainEnglishSummary};
use loom_types::session::SessionSnapshot;

pub fn project_session_turns(session: &SessionSnapshot) -> Vec<SessionTurn> {
    let mut turns = Vec::new();
    if let Some(intent) = &session.intent {
        turns.push(intent_turn(session, intent));
        turns.extend(clarify_turns(session, intent));
    }
    turns.extend(op_turns(session));
    turns.sort_by(|left, right| turn_at(left).cmp(turn_at(right)));
    turns
}

fn intent_turn(session: &SessionSnapshot, intent: &IntentPacket) -> SessionTurn {
    let (source_kind, raw) = match &intent.source {
        IntentSource::Text { raw } => ("text", raw.clone()),
        IntentSource::Voice { transcript } => ("voice", transcript.clone()),
        IntentSource::Spec { raw } => ("spec", raw.clone()),
        IntentSource::Incident { path } => ("incident", path.clone()),
        IntentSource::Sketch { path } => ("sketch", path.clone()),
        IntentSource::Image { path, .. } => ("image", path.clone()),
    };
    let at = session
        .op_log
        .iter()
        .find(|op| op.op == "intent.formalize")
        .map(|op| op.started_at.clone())
        .unwrap_or_else(|| "2000-01-01T00:00:00Z".to_string());
    SessionTurn::UserIntent {
        id: stable_turn_id(session.session_id, "intent"),
        at,
        raw,
        source_kind: source_kind.to_string(),
    }
}

fn clarify_turns(session: &SessionSnapshot, intent: &IntentPacket) -> Vec<SessionTurn> {
    let mut by_round: BTreeMap<(u32, String), Vec<_>> = BTreeMap::new();
    for turn in &intent.clarification_history {
        by_round
            .entry((turn.round, turn.agent_id.clone()))
            .or_default()
            .push(turn);
    }
    let mut turns = Vec::new();
    for ((round, agent_id), questions) in by_round {
        let at = questions
            .first()
            .map(|turn| turn.question_recorded_at.clone())
            .unwrap_or_else(|| "2000-01-01T00:00:00Z".to_string());
        let rendered = questions
            .iter()
            .map(|question| TurnQuestion {
                id: question.question_id.clone(),
                text: question.question_text.clone(),
                witness_kind: question.witness_kind.clone(),
                options: question.options.clone(),
                answer: question.answer_text.clone(),
            })
            .collect::<Vec<_>>();
        turns.push(SessionTurn::AgentClarify {
            id: stable_turn_id(session.session_id, &format!("clarify:{round}:{agent_id}")),
            at,
            agent_id: agent_id.clone(),
            questions: rendered,
        });
        for question in questions {
            if let Some(answer) = &question.answer_text {
                turns.push(SessionTurn::UserAnswer {
                    id: stable_turn_id(
                        session.session_id,
                        &format!("answer:{}:{}", question.question_id, answer),
                    ),
                    at: question
                        .answer_recorded_at
                        .clone()
                        .unwrap_or_else(|| question.question_recorded_at.clone()),
                    question_id: question.question_id.clone(),
                    text: answer.clone(),
                });
            }
        }
    }
    turns
}

fn op_turns(session: &SessionSnapshot) -> Vec<SessionTurn> {
    let mut turns = Vec::new();
    let mut build_group: Vec<&OpRecord> = Vec::new();
    for op in &session.op_log {
        if op.op.starts_with("build.stage.") {
            build_group.push(op);
            continue;
        }
        flush_build_group(&mut turns, session.session_id, &mut build_group);
        if op.op == "summary.plain_english" {
            if let Some(summary) = summary_from_op(op) {
                let mut op_ids = vec![op.id];
                if let Some(verify) = session
                    .op_log
                    .iter()
                    .rev()
                    .find(|candidate| candidate.op == "xtal.verify")
                {
                    op_ids.push(verify.id);
                }
                turns.push(SessionTurn::Verified {
                    id: stable_turn_id(session.session_id, &format!("verified:{}", op.id)),
                    at: op.started_at.clone(),
                    summary,
                    op_ids,
                });
            }
        } else if op.op.starts_with("agent.event.") && op.op.contains(".incident.") {
            let incident_id = incident_id_from_op(op).unwrap_or_else(|| op.id.to_string());
            turns.push(SessionTurn::Incident {
                id: stable_turn_id(session.session_id, &format!("incident:{incident_id}")),
                at: op.started_at.clone(),
                incident_id,
                summary: op
                    .notes
                    .clone()
                    .or_else(|| op.stdout.clone())
                    .unwrap_or_else(|| "Runtime incident detected.".to_string()),
                repair_available: true,
            });
        } else if op.op.starts_with("xtal.improve") || op.op.starts_with("xtal.repair") {
            turns.push(SessionTurn::Repair {
                id: stable_turn_id(session.session_id, &format!("repair:{}", op.id)),
                at: op.started_at.clone(),
                incident_id: incident_id_from_op(op).unwrap_or_else(|| "latest".to_string()),
                op_ids: vec![op.id],
            });
        } else if op.op.starts_with("agent.event.") && op.op.contains(".stream_") {
            if let Some((agent_id, event)) = stream_event_from_op(op) {
                turns.push(SessionTurn::AgentStream {
                    id: stable_turn_id(session.session_id, &format!("agent-stream:{}", op.id)),
                    at: op.started_at.clone(),
                    agent_id,
                    event,
                    op_id: op.id,
                });
            }
        } else if op.op == "agent.event.quorum.realize" {
            if let Some(round) = realize_quorum_from_op(op) {
                turns.push(SessionTurn::QuorumRealize {
                    id: stable_turn_id(session.session_id, &format!("quorum-realize:{}", op.id)),
                    at: op.started_at.clone(),
                    round,
                    op_ids: vec![op.id],
                });
            }
        } else if op.op.starts_with("agent.realize.") {
            let agent_id = op
                .op
                .strip_prefix("agent.realize.")
                .unwrap_or("agent")
                .to_string();
            let wrote_files: Vec<String> = op
                .report_json
                .as_ref()
                .and_then(|value| value.get("write_audit"))
                .map(|audit| {
                    let mut files: Vec<String> = audit
                        .get("created")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| item.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    if let Some(modified) = audit.get("modified").and_then(|v| v.as_array()) {
                        for item in modified {
                            if let Some(text) = item.as_str() {
                                files.push(text.to_string());
                            }
                        }
                    }
                    files
                })
                .unwrap_or_default();
            turns.push(SessionTurn::AgentRealize {
                id: stable_turn_id(session.session_id, &format!("agent-realize:{}", op.id)),
                at: op.started_at.clone(),
                agent_id,
                ok: matches!(op.status, loom_types::artifacts::OperationStatus::Succeeded),
                wrote_files,
                op_ids: vec![op.id],
            });
        } else if op.op.starts_with("agent.handoff.") || op.op.starts_with("agent.run.") {
            let agent_id = op.op.rsplit('.').next().unwrap_or("agent").to_string();
            turns.push(SessionTurn::AgentDraft {
                id: stable_turn_id(session.session_id, &format!("agent-draft:{}", op.id)),
                at: op.started_at.clone(),
                agent_id,
                summary: op
                    .notes
                    .clone()
                    .or_else(|| op.stdout.clone())
                    .unwrap_or_else(|| op.op.clone()),
                evidence: op
                    .artifacts
                    .iter()
                    .map(|artifact| TurnEvidence {
                        label: artifact.clone(),
                        op_id: Some(op.id),
                        artifact: Some(artifact.clone()),
                    })
                    .collect(),
            });
        } else if op.op == "intent.clarify.answers" {
            continue;
        } else if op.op == "spec.approve" {
            turns.push(SessionTurn::UserApproved {
                id: stable_turn_id(session.session_id, &format!("approved:{}", op.id)),
                at: op.started_at.clone(),
                by: "human".to_string(),
            });
        }
    }
    flush_build_group(&mut turns, session.session_id, &mut build_group);
    turns
}

fn flush_build_group(
    turns: &mut Vec<SessionTurn>,
    session_id: Uuid,
    build_group: &mut Vec<&OpRecord>,
) {
    if build_group.is_empty() {
        return;
    }
    let first = build_group[0];
    let stage = build_group
        .last()
        .map(|op| op.op.trim_start_matches("build.stage.").to_string())
        .unwrap_or_else(|| "start".to_string());
    let ids = build_group.iter().map(|op| op.id).collect::<Vec<_>>();
    turns.push(SessionTurn::BuildStage {
        id: stable_turn_id(session_id, &format!("build:{}:{}", first.id, stage)),
        at: first.started_at.clone(),
        stage,
        op_ids: ids,
    });
    build_group.clear();
}

fn summary_from_op(op: &OpRecord) -> Option<PlainEnglishSummary> {
    op.report_json
        .as_ref()
        .and_then(|value| serde_json::from_value(value.clone()).ok())
}

fn incident_id_from_op(op: &OpRecord) -> Option<String> {
    op.report_json
        .as_ref()
        .and_then(|value| value.get("incident_id"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| op.report_json.as_ref()?.get("id")?.as_str())
        .map(ToOwned::to_owned)
}

fn turn_at(turn: &SessionTurn) -> &str {
    match turn {
        SessionTurn::UserIntent { at, .. }
        | SessionTurn::AgentClarify { at, .. }
        | SessionTurn::UserAnswer { at, .. }
        | SessionTurn::AgentDraft { at, .. }
        | SessionTurn::UserApproved { at, .. }
        | SessionTurn::BuildStage { at, .. }
        | SessionTurn::Verified { at, .. }
        | SessionTurn::Incident { at, .. }
        | SessionTurn::Repair { at, .. }
        | SessionTurn::AgentRealize { at, .. }
        | SessionTurn::AgentStream { at, .. }
        | SessionTurn::QuorumRealize { at, .. } => at,
    }
}

fn stream_event_from_op(op: &OpRecord) -> Option<(String, AgentStreamEvent)> {
    let agent_id = op
        .report_json
        .as_ref()
        .and_then(|value| value.get("agent_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            let rest = op.op.strip_prefix("agent.event.")?;
            rest.split(".stream_").next().map(str::to_string)
        })?;
    let event = op
        .report_json
        .as_ref()
        .and_then(|value| value.get("event"))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())?;
    Some((agent_id, event))
}

fn realize_quorum_from_op(op: &OpRecord) -> Option<RealizeQuorumRound> {
    op.report_json
        .as_ref()
        .and_then(|value| value.get("round"))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

fn stable_turn_id(session_id: Uuid, key: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(session_id.as_bytes());
    hasher.update(key.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use loom_types::artifacts::{
        ClarificationTurn, IntentPacket, OpRecord, OperationStatus, TaskType, WitnessKind,
    };
    use loom_types::session::SessionSnapshot;

    use super::project_session_turns;

    fn op(session_id: Uuid, id: Uuid, name: &str) -> OpRecord {
        OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id,
            session_id,
            op: name.to_string(),
            backend: "test".to_string(),
            command: Vec::new(),
            started_at: format!("2026-05-12T12:00:{:02}Z", id.as_bytes()[0]),
            finished_at: None,
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
    fn intent_only_projects_one_turn() {
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", "/workspace", TaskType::NewBehavior);
        session.intent = Some(IntentPacket::demo(session_id, "/workspace"));

        let turns = project_session_turns(&session);

        assert_eq!(turns.len(), 1);
        assert!(matches!(
            turns.first(),
            Some(loom_types::api::SessionTurn::UserIntent { .. })
        ));
    }

    #[test]
    fn clarify_questions_and_answers_project_to_turns() {
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", "/workspace", TaskType::NewBehavior);
        let mut intent = IntentPacket::demo(session_id, "/workspace");
        intent.clarification_history = vec![ClarificationTurn {
            question_id: "q1".to_string(),
            question_text: "What about empty input?".to_string(),
            witness_kind: WitnessKind::ForbiddenBehavior,
            round: 1,
            agent_id: "claude-code".to_string(),
            options: vec!["Reject".to_string()],
            question_recorded_at: "2026-05-12T12:00:01Z".to_string(),
            answer_text: Some("Reject".to_string()),
            answer_recorded_at: Some("2026-05-12T12:00:02Z".to_string()),
        }];
        session.intent = Some(intent);

        let turns = project_session_turns(&session);

        assert_eq!(turns.len(), 3);
    }

    #[test]
    fn build_stages_collapse() {
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", "/workspace", TaskType::NewBehavior);
        session.op_log = vec![
            op(session_id, Uuid::from_u128(1), "build.stage.start"),
            op(session_id, Uuid::from_u128(2), "build.stage.done"),
        ];

        let turns = project_session_turns(&session);

        assert_eq!(turns.len(), 1);
        assert!(matches!(
            turns.first(),
            Some(loom_types::api::SessionTurn::BuildStage { .. })
        ));
    }

    #[test]
    fn summary_projects_verified_turn() {
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", "/workspace", TaskType::NewBehavior);
        let mut summary = op(session_id, Uuid::from_u128(7), "summary.plain_english");
        summary.report_json = Some(serde_json::json!({
            "schema_version": "x07.studio.plain_english_summary@0.1.0",
            "headline": "Built and verified.",
            "behavior_promises": [],
            "boundaries": [],
            "evidence": [],
            "run_invocation": null,
            "followups": []
        }));
        session.op_log = vec![summary];

        let turns = project_session_turns(&session);

        assert!(matches!(
            turns.first(),
            Some(loom_types::api::SessionTurn::Verified { .. })
        ));
    }
}
