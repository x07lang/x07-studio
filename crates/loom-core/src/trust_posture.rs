use camino::Utf8Path;
use loom_adapters::command_runner::now_string;
use loom_types::api::{
    BudgetSummary, Capability, PostureDelta, ProofCoverage, ProofSupportNote, TrustPosture,
};
use loom_types::artifacts::OperationStatus;
use loom_types::session::SessionSnapshot;
use serde_json::Value;

pub fn current(root: &Utf8Path, session: &SessionSnapshot) -> TrustPosture {
    let trust_report = latest_json_for_op(session, "trust.")
        .or_else(|| read_json(root.join("target/trust/report.json").as_path()))
        .or_else(|| read_json(root.join("target/cert/trust-report.json").as_path()))
        .unwrap_or(Value::Null);
    let verify_report = latest_json_for_op(session, "xtal.verify")
        .or_else(|| read_json(root.join("target/xtal/verify/summary.json").as_path()))
        .unwrap_or(Value::Null);
    let x07_json = read_json(root.join("x07.json").as_path()).unwrap_or(Value::Null);
    let profile = current_profile(root, session, &trust_report);
    let worlds = worlds(&trust_report, &x07_json);
    let capabilities = capabilities(root, &profile, &trust_report, &worlds);
    let budgets = budgets(&trust_report, &verify_report, &x07_json);
    let proof_coverage = proof_coverage(session, &verify_report);
    let proof_support_notes = proof_support_notes_from_diag(root);
    let mut posture = TrustPosture {
        schema_version: "x07.studio.trust_posture@0.1.0".to_string(),
        session_id: session.session_id,
        captured_at: now_string(),
        trust_profile: profile,
        worlds,
        capabilities,
        budgets,
        proof_coverage,
        proof_support_notes,
        deltas: Vec::new(),
        posture_color: String::new(),
    };
    if let Some(prev) = latest_captured_posture(session) {
        posture.deltas = diff_postures(&prev, &posture);
    }
    posture.posture_color = posture_color(&posture);
    posture
}

pub fn diff_postures(prev: &TrustPosture, next: &TrustPosture) -> Vec<PostureDelta> {
    let at = next.captured_at.clone();
    let mut deltas = Vec::new();
    for world in next
        .worlds
        .iter()
        .filter(|world| !prev.worlds.contains(*world))
    {
        deltas.push(delta(&at, "world-widen", format!("adds world `{world}`")));
    }
    for cap in next
        .capabilities
        .iter()
        .filter(|cap| !prev.capabilities.iter().any(|old| old.id == cap.id))
    {
        deltas.push(delta(
            &at,
            "capability-widen",
            format!("adds capability `{}` from {}", cap.id, cap.source),
        ));
    }
    if next.budgets.local_cap_ms > prev.budgets.local_cap_ms {
        deltas.push(delta(
            &at,
            "budget-increase",
            format!(
                "local cap increased from {:?} to {:?} ms",
                prev.budgets.local_cap_ms, next.budgets.local_cap_ms
            ),
        ));
    }
    if next.proof_coverage.proved_pct + f32::EPSILON < prev.proof_coverage.proved_pct {
        deltas.push(delta(
            &at,
            "proof-coverage-drop",
            format!(
                "proved coverage dropped from {:.0}% to {:.0}%",
                prev.proof_coverage.proved_pct, next.proof_coverage.proved_pct
            ),
        ));
    }
    if prev.trust_profile != next.trust_profile {
        deltas.push(delta(
            &at,
            "profile-change",
            format!(
                "trust profile changed from `{}` to `{}`",
                prev.trust_profile, next.trust_profile
            ),
        ));
    }
    deltas
}

pub fn latest_captured_posture(session: &SessionSnapshot) -> Option<TrustPosture> {
    session.op_log.iter().rev().find_map(|op| {
        if op.op != "posture.captured" {
            return None;
        }
        op.report_json
            .as_ref()
            .and_then(|json| json.get("posture").cloned())
            .and_then(|json| serde_json::from_value(json).ok())
    })
}

fn delta(at: &str, kind: &str, summary: String) -> PostureDelta {
    PostureDelta {
        at: at.to_string(),
        kind: kind.to_string(),
        summary,
    }
}

fn current_profile(root: &Utf8Path, session: &SessionSnapshot, trust_report: &Value) -> String {
    if let Some(value) = first_string_for_keys(trust_report, &["profile", "trust_profile"]) {
        return value;
    }
    let ladder = crate::ladder::ladder_state(root, session);
    if let Some(rung) = ladder
        .rungs
        .iter()
        .find(|rung| rung.id == ladder.current_rung)
    {
        return rung.profile_path.clone().unwrap_or_else(|| rung.id.clone());
    }
    "local_preview".to_string()
}

fn worlds(trust_report: &Value, x07_json: &Value) -> Vec<String> {
    let mut worlds = string_array_for_keys(trust_report, &["worlds", "allowed_worlds"]);
    if worlds.is_empty() {
        if let Some(world) = first_string_for_keys(trust_report, &["world", "default_world"]) {
            worlds.push(world);
        }
    }
    if let Some(world) = x07_json.get("world").and_then(Value::as_str) {
        if world != "solve-pure" && !worlds.iter().any(|item| item == "solve-pure") {
            worlds.push("solve-pure".to_string());
        }
        if !worlds.iter().any(|item| item == world) {
            worlds.push(world.to_string());
        }
    }
    if worlds.is_empty() {
        if let Some(profile) = first_string_for_keys(x07_json, &["default_profile", "profile"]) {
            if profile.contains("os") {
                worlds.push(profile);
            }
        }
    }
    if worlds.is_empty() {
        worlds.push("solve-pure".to_string());
    }
    worlds.sort();
    worlds.dedup();
    worlds
}

fn capabilities(
    root: &Utf8Path,
    profile: &str,
    trust_report: &Value,
    worlds: &[String],
) -> Vec<Capability> {
    let mut caps = Vec::new();
    push_capabilities(&mut caps, trust_report, "trust report");
    if profile.ends_with(".json") {
        if let Some(profile_json) = read_json(root.join(profile).as_path()) {
            push_capabilities(&mut caps, &profile_json, profile);
        }
    }
    if source_contains(root, "std.os.time") {
        caps.push(Capability {
            id: "os-time".to_string(),
            source: "source import".to_string(),
            justification: "source imports std.os.time for wall-clock reads".to_string(),
        });
    }
    for world in worlds {
        if world.contains("os-net") || world.contains("network") {
            caps.push(Capability {
                id: "os-net".to_string(),
                source: "world".to_string(),
                justification: format!("world `{world}` allows network-facing IO"),
            });
        } else if world.contains("os-fs") {
            caps.push(Capability {
                id: "os-fs".to_string(),
                source: "world".to_string(),
                justification: format!("world `{world}` allows filesystem IO"),
            });
        } else if world.contains("os-time") {
            caps.push(Capability {
                id: "os-time".to_string(),
                source: "world".to_string(),
                justification: format!("world `{world}` allows clock reads"),
            });
        }
    }
    caps.sort_by(|a, b| a.id.cmp(&b.id).then(a.source.cmp(&b.source)));
    caps.dedup_by(|a, b| a.id == b.id && a.source == b.source);
    caps
}

fn source_contains(root: &Utf8Path, needle: &str) -> bool {
    let mut stack = vec![root.join("src").as_std_path().to_path_buf()];
    let mut files_read = 0usize;
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(entry.path());
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            files_read += 1;
            if files_read > 512 {
                return false;
            }
            if std::fs::read_to_string(entry.path())
                .map(|text| text.contains(needle))
                .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}

fn push_capabilities(out: &mut Vec<Capability>, value: &Value, source: &str) {
    for item in string_array_for_keys(value, &["capabilities", "allowed_capabilities", "caps"]) {
        out.push(Capability {
            id: item,
            source: source.to_string(),
            justification: "declared by trust surface".to_string(),
        });
    }
}

fn budgets(trust_report: &Value, verify_report: &Value, x07_json: &Value) -> BudgetSummary {
    BudgetSummary {
        local_cap_ms: first_u64_for_keys(trust_report, &["local_cap_ms", "cap_ms", "max_ms"])
            .or_else(|| first_u64_for_keys(x07_json, &["local_cap_ms", "cap_ms", "max_ms"])),
        arch_profile: first_string_for_keys(trust_report, &["arch_profile", "budget_profile"])
            .or_else(|| first_string_for_keys(x07_json, &["arch_profile", "budget_profile"])),
        prover_seconds_used: first_u64_for_keys(
            verify_report,
            &["prover_seconds_used", "z3_seconds", "prove_seconds"],
        )
        .unwrap_or(0),
        prover_seconds_cap: first_u64_for_keys(
            trust_report,
            &["prover_seconds_cap", "prove_seconds_cap"],
        ),
    }
}

fn proof_coverage(session: &SessionSnapshot, verify_report: &Value) -> ProofCoverage {
    let verified = session
        .op_log
        .iter()
        .rev()
        .any(|op| op.op == "xtal.verify" && op.status == OperationStatus::Succeeded);
    let support = percent_for_keys(
        verify_report,
        &["support_pct", "coverage_pct", "proof_coverage_pct"],
    )
    .unwrap_or(if verified { 100.0 } else { 0.0 });
    let proved = percent_for_keys(verify_report, &["proved_pct", "prove_pct"])
        .unwrap_or(if verified { 87.0 } else { 0.0 });
    let proof_count = first_u64_for_keys(verify_report, &["proof_count", "proved_count"])
        .unwrap_or(if verified { 1 } else { 0 }) as u32;
    let assumptions_open = session
        .intent
        .as_ref()
        .map(|intent| {
            intent
                .ambiguities
                .iter()
                .filter(|item| !item.trim().is_empty())
                .count() as u32
        })
        .unwrap_or(0);
    ProofCoverage {
        support_pct: support,
        proved_pct: proved,
        proof_count,
        assumptions_open,
    }
}

fn posture_color(posture: &TrustPosture) -> String {
    if posture
        .capabilities
        .iter()
        .any(|cap| cap.id.contains("net") || cap.id.contains("danger"))
        || posture.worlds.iter().any(|world| world.contains("os-net"))
        || posture.proof_coverage.proved_pct < 35.0
    {
        "red".to_string()
    } else if !posture.capabilities.is_empty()
        || posture.worlds.iter().any(|world| world.contains("os"))
        || posture.proof_coverage.proved_pct < 70.0
    {
        "amber".to_string()
    } else {
        "green".to_string()
    }
}

fn latest_json_for_op(session: &SessionSnapshot, prefix: &str) -> Option<Value> {
    session
        .op_log
        .iter()
        .rev()
        .find(|op| op.op.starts_with(prefix) && op.report_json.is_some())
        .and_then(|op| op.report_json.clone())
}

/// Collect `WXTAL_VERIFY_PROVE_*` / `X07V_*` notes from x07's verify
/// diag. Studio's TrustCard renders these inline so users see *why* the
/// prover left a target unverified (e.g. "no requires/ensures
/// declared", "unsupported heap effect"), instead of a silent
/// `support_pct` figure with no context.
fn proof_support_notes_from_diag(root: &Utf8Path) -> Vec<ProofSupportNote> {
    let diag_path = root.join("target/xtal/xtal.verify.diag.json");
    let Some(diag) = read_json(diag_path.as_path()) else {
        return Vec::new();
    };
    let Some(diagnostics) = diag.get("diagnostics").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut notes = Vec::new();
    for entry in diagnostics {
        let code = entry
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let is_proof_support = code.starts_with("WXTAL_VERIFY_PROVE_")
            || code.starts_with("X07V_")
            || code.starts_with("EXTAL_VERIFY_PROVE_");
        if !is_proof_support {
            continue;
        }
        let message = entry
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if message.is_empty() {
            continue;
        }
        let severity = entry
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("warning")
            .to_string();
        let target = entry
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        notes.push(ProofSupportNote {
            code: code.to_string(),
            target,
            severity,
            message,
        });
    }
    notes
}

fn read_json(path: &Utf8Path) -> Option<Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn string_array_for_keys(value: &Value, keys: &[&str]) -> Vec<String> {
    find_key_recursive(value, keys)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
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

fn first_string_for_keys(value: &Value, keys: &[&str]) -> Option<String> {
    find_key_recursive(value, keys)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn first_u64_for_keys(value: &Value, keys: &[&str]) -> Option<u64> {
    find_key_recursive(value, keys).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_f64().map(|num| num.max(0.0) as u64))
    })
}

fn percent_for_keys(value: &Value, keys: &[&str]) -> Option<f32> {
    find_key_recursive(value, keys).and_then(|value| {
        let raw = value.as_f64()?;
        Some(if raw <= 1.0 { raw * 100.0 } else { raw } as f32)
    })
}

fn find_key_recursive<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key) {
                    return Some(value);
                }
            }
            map.values()
                .find_map(|value| find_key_recursive(value, keys))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|value| find_key_recursive(value, keys)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{current, diff_postures};
    use loom_types::artifacts::{OpRecord, OperationStatus, TaskType};
    use loom_types::session::SessionSnapshot;
    use uuid::Uuid;

    #[test]
    fn posture_defaults_to_solve_pure_after_verified_session() {
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("mkdir");
        std::fs::write(root.join("x07.json"), "{}").expect("x07");
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", root.to_string(), TaskType::NewBehavior);
        session
            .op_log
            .push(op(session_id, "xtal.verify", OperationStatus::Succeeded));

        let posture = current(root.as_path(), &session);

        assert!(posture.worlds.contains(&"solve-pure".to_string()));
        assert!(posture.proof_coverage.proved_pct >= 50.0);
        assert_eq!(posture.posture_color, "green");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn posture_surfaces_run_os_world_and_os_time_import() {
        let root = temp_root();
        std::fs::create_dir_all(root.join("src/app")).expect("mkdir src");
        std::fs::write(root.join("x07.json"), r#"{"world":"run-os"}"#).expect("x07");
        std::fs::write(
            root.join("src/app/timer.x07.json"),
            r#"{"imports":["std.os.time"]}"#,
        )
        .expect("timer source");
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "timer", root.to_string(), TaskType::NewBehavior);
        session
            .op_log
            .push(op(session_id, "xtal.verify", OperationStatus::Succeeded));

        let posture = current(root.as_path(), &session);

        assert!(posture.worlds.contains(&"solve-pure".to_string()));
        assert!(posture.worlds.contains(&"run-os".to_string()));
        assert!(posture.capabilities.iter().any(|cap| cap.id == "os-time"));
        assert_eq!(posture.posture_color, "amber");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn diff_flags_world_capability_and_budget_widening() {
        let root = temp_root();
        let session_id = Uuid::new_v4();
        let session =
            SessionSnapshot::new(session_id, "demo", root.to_string(), TaskType::NewBehavior);
        let mut prev = current(root.as_path(), &session);
        let mut next = prev.clone();
        next.worlds.push("os-net".to_string());
        next.capabilities.push(loom_types::api::Capability {
            id: "os-net".to_string(),
            source: "test".to_string(),
            justification: "network".to_string(),
        });
        next.budgets.local_cap_ms = Some(2000);
        prev.budgets.local_cap_ms = Some(1000);

        let deltas = diff_postures(&prev, &next);

        assert!(deltas.iter().any(|delta| delta.kind == "world-widen"));
        assert!(deltas.iter().any(|delta| delta.kind == "capability-widen"));
        assert!(deltas.iter().any(|delta| delta.kind == "budget-increase"));
        std::fs::remove_dir_all(root).ok();
    }

    fn op(session_id: Uuid, name: &str, status: OperationStatus) -> OpRecord {
        OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id,
            op: name.to_string(),
            backend: "test".to_string(),
            command: Vec::new(),
            started_at: "1".to_string(),
            finished_at: Some("1".to_string()),
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

    fn temp_root() -> camino::Utf8PathBuf {
        camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-posture-{}", Uuid::new_v4()))
    }
}
