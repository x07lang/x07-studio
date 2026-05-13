use camino::Utf8Path;
use loom_types::api::{DiffRef, SemanticDiff, SemanticDiffRequest, TrustPosture};
use loom_types::session::SessionSnapshot;
use serde_json::Value;

pub fn between(
    root: &Utf8Path,
    session: &SessionSnapshot,
    request: SemanticDiffRequest,
) -> SemanticDiff {
    let from_surface = surface_for_ref(root, session, &request.from);
    let to_surface = surface_for_ref(root, session, &request.to);
    let world_changes = added("world", &from_surface.worlds, &to_surface.worlds);
    let capability_changes = added(
        "capability",
        &from_surface.capabilities,
        &to_surface.capabilities,
    );
    let budget_changes = if to_surface.budget_score > from_surface.budget_score {
        vec![format!(
            "budget score increased from {} to {}",
            from_surface.budget_score, to_surface.budget_score
        )]
    } else {
        Vec::new()
    };
    let proof_changes = if to_surface.proved_pct + f32::EPSILON < from_surface.proved_pct {
        vec![format!(
            "proved coverage dropped from {:.0}% to {:.0}%",
            from_surface.proved_pct, to_surface.proved_pct
        )]
    } else if to_surface.proved_pct > from_surface.proved_pct + f32::EPSILON {
        vec![format!(
            "proved coverage increased from {:.0}% to {:.0}%",
            from_surface.proved_pct, to_surface.proved_pct
        )]
    } else {
        Vec::new()
    };
    let trust_delta_color = diff_color(
        &world_changes,
        &capability_changes,
        &budget_changes,
        &proof_changes,
    );
    let headline = headline(
        &world_changes,
        &capability_changes,
        &budget_changes,
        &proof_changes,
        &to_surface,
    );
    SemanticDiff {
        schema_version: "x07.studio.semantic_diff@0.1.0".to_string(),
        from: request.from,
        to: request.to,
        headline,
        trust_delta_color,
        raw: serde_json::json!({
            "schema_version": "x07.review.diff@0.5.0",
            "mode": request.mode,
            "from": from_surface.raw,
            "to": to_surface.raw,
            "world_changes": world_changes,
            "capability_changes": capability_changes,
            "budget_changes": budget_changes,
            "proof_changes": proof_changes,
        }),
        world_changes,
        capability_changes,
        budget_changes,
        proof_changes,
    }
}

#[derive(Debug)]
struct DiffSurface {
    worlds: Vec<String>,
    capabilities: Vec<String>,
    budget_score: u64,
    proved_pct: f32,
    raw: Value,
}

fn surface_for_ref(root: &Utf8Path, session: &SessionSnapshot, reference: &DiffRef) -> DiffSurface {
    match reference {
        DiffRef::Current => from_posture(crate::trust_posture::current(root, session)),
        DiffRef::OpId { op_id } | DiffRef::TurnId { turn_id: op_id } => session
            .op_log
            .iter()
            .find(|op| &op.id == op_id)
            .map(|op| {
                let raw = op.report_json.clone().unwrap_or_else(|| {
                    serde_json::json!({
                        "op": op.op,
                        "status": op.status,
                        "artifacts": op.artifacts,
                        "notes": op.notes,
                    })
                });
                from_raw(raw)
            })
            .unwrap_or_else(empty_surface),
        DiffRef::Hash { hash } => from_raw(serde_json::json!({ "hash": hash })),
        DiffRef::QuorumProposal { agent_id, .. } => session
            .op_log
            .iter()
            .rev()
            .find_map(|op| {
                let round = op.report_json.as_ref()?.get("round")?;
                let proposals = round.get("proposals")?.as_array()?;
                proposals.iter().find(|proposal| {
                    proposal.get("agent_id").and_then(Value::as_str) == Some(agent_id.as_str())
                })
            })
            .cloned()
            .map(from_raw)
            .unwrap_or_else(empty_surface),
    }
}

fn from_posture(posture: TrustPosture) -> DiffSurface {
    let raw = serde_json::to_value(&posture).unwrap_or(Value::Null);
    DiffSurface {
        worlds: posture.worlds.clone(),
        capabilities: posture
            .capabilities
            .iter()
            .map(|capability| capability.id.clone())
            .collect(),
        budget_score: posture
            .budgets
            .local_cap_ms
            .unwrap_or(posture.budgets.prover_seconds_cap.unwrap_or(0) * 1000),
        proved_pct: posture.proof_coverage.proved_pct,
        raw,
    }
}

fn from_raw(raw: Value) -> DiffSurface {
    let text = raw.to_string().to_ascii_lowercase();
    let mut worlds = string_array(&raw, &["worlds", "allowed_worlds"]);
    if worlds.is_empty() && text.contains("solve-pure") {
        worlds.push("solve-pure".to_string());
    }
    if text.contains("os-net") || text.contains("network") {
        worlds.push("os-net".to_string());
    }
    if text.contains("os-fs") || text.contains("filesystem") {
        worlds.push("os-fs".to_string());
    }
    if worlds.is_empty() {
        worlds.push("solve-pure".to_string());
    }
    worlds.sort();
    worlds.dedup();

    let mut capabilities = string_array(&raw, &["capabilities", "allowed_capabilities", "caps"]);
    if text.contains("os-net") || text.contains("network") {
        capabilities.push("os-net".to_string());
    }
    if text.contains("os-fs") || text.contains("filesystem") {
        capabilities.push("os-fs".to_string());
    }
    capabilities.sort();
    capabilities.dedup();

    DiffSurface {
        worlds,
        capabilities,
        budget_score: first_u64(&raw, &["local_cap_ms", "cap_ms", "budget_score"]).unwrap_or(0),
        proved_pct: percent(&raw, &["proved_pct", "proof_coverage_pct"]).unwrap_or(0.0),
        raw,
    }
}

fn empty_surface() -> DiffSurface {
    DiffSurface {
        worlds: vec!["solve-pure".to_string()],
        capabilities: Vec::new(),
        budget_score: 0,
        proved_pct: 0.0,
        raw: Value::Null,
    }
}

fn added(label: &str, before: &[String], after: &[String]) -> Vec<String> {
    after
        .iter()
        .filter(|item| !before.contains(*item))
        .map(|item| format!("adds {label} `{item}`"))
        .collect()
}

fn diff_color(
    world_changes: &[String],
    capability_changes: &[String],
    budget_changes: &[String],
    proof_changes: &[String],
) -> String {
    if world_changes.iter().any(|item| item.contains("os-net"))
        || capability_changes
            .iter()
            .any(|item| item.contains("os-net"))
        || proof_changes.iter().any(|item| item.contains("dropped"))
    {
        "red".to_string()
    } else if !world_changes.is_empty()
        || !capability_changes.is_empty()
        || !budget_changes.is_empty()
    {
        "amber".to_string()
    } else {
        "green".to_string()
    }
}

fn headline(
    world_changes: &[String],
    capability_changes: &[String],
    budget_changes: &[String],
    proof_changes: &[String],
    to_surface: &DiffSurface,
) -> String {
    let mut parts = Vec::new();
    parts.extend(world_changes.iter().cloned());
    parts.extend(capability_changes.iter().cloned());
    parts.extend(budget_changes.iter().cloned());
    parts.extend(proof_changes.iter().cloned());
    if parts.is_empty() {
        if to_surface.worlds.iter().any(|world| world == "solve-pure") {
            "stays solve-pure · no trust delta".to_string()
        } else {
            "no semantic trust delta".to_string()
        }
    } else {
        parts.join(" · ")
    }
}

fn string_array(value: &Value, keys: &[&str]) -> Vec<String> {
    find_key(value, keys)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    Value::String(text) => Some(text.clone()),
                    Value::Object(map) => map
                        .get("id")
                        .or_else(|| map.get("name"))
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn first_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    find_key(value, keys).and_then(|value| value.as_u64())
}

fn percent(value: &Value, keys: &[&str]) -> Option<f32> {
    find_key(value, keys).and_then(|value| {
        let raw = value.as_f64()?;
        Some(if raw <= 1.0 { raw * 100.0 } else { raw } as f32)
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
    use super::between;
    use loom_types::api::{DiffRef, SemanticDiffRequest};
    use loom_types::artifacts::{OpRecord, OperationStatus, TaskType};
    use loom_types::session::SessionSnapshot;
    use uuid::Uuid;

    #[test]
    fn semantic_diff_flags_network_capability() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-semantic-diff-{}", Uuid::new_v4()));
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", root.to_string(), TaskType::NewBehavior);
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        session.op_log.push(op(
            session_id,
            from,
            serde_json::json!({"worlds":["solve-pure"]}),
        ));
        session.op_log.push(op(
            session_id,
            to,
            serde_json::json!({"worlds":["solve-pure","os-net"]}),
        ));

        let diff = between(
            root.as_path(),
            &session,
            SemanticDiffRequest {
                schema_version: "x07.studio.semantic_diff_request@0.1.0".to_string(),
                from: DiffRef::OpId { op_id: from },
                to: DiffRef::OpId { op_id: to },
                mode: "project".to_string(),
            },
        );

        assert_eq!(diff.trust_delta_color, "red");
        assert!(diff.headline.contains("os-net"));
        std::fs::remove_dir_all(root).ok();
    }

    fn op(session_id: Uuid, id: Uuid, report_json: serde_json::Value) -> OpRecord {
        OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id,
            session_id,
            op: "test".to_string(),
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
            report_json: Some(report_json),
            report_path: None,
        }
    }
}
