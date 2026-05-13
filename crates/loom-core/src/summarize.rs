//! Deterministic plain-English summarizer over an approved intent and the
//! latest verify evidence. Emits `summary.plain_english` content that the
//! Simple-Mode UI renders as a one-paragraph behavior promise plus a few
//! supporting bullets ("Verified on N example cases", "Rejects empty input",
//! etc.) for non-engineer review.
//!
//! No LLM call — the summary is derived from the intent packet, spec
//! operations, witness list, and a bounded read of
//! `target/xtal/verify/summary.json`.

use camino::Utf8Path;
use loom_types::artifacts::{IntentPacket, PlainEnglishSummary, TaskType, WitnessKind};
use loom_types::session::{SessionPhase, SessionSnapshot};

pub fn plain_english_summary_from_session(
    session: &SessionSnapshot,
) -> Option<PlainEnglishSummary> {
    plain_english_summary_with_root(session, None)
}

/// Variant that also inspects the on-disk `src/` tree under `root` so the
/// summary can flag scaffold-only implementations. Used by the kernel
/// which already knows the workspace path; the no-arg form keeps the old
/// signature for tests and projection-only consumers.
pub fn plain_english_summary_with_root(
    session: &SessionSnapshot,
    root: Option<&Utf8Path>,
) -> Option<PlainEnglishSummary> {
    let intent = session.intent.as_ref()?;
    let stub_paths = root.map(scan_stub_modules).unwrap_or_default();
    let scaffold_only = !stub_paths.is_empty();
    let headline = headline_for(session, intent, scaffold_only);
    let behavior_promises = behavior_promises_for(intent);
    let behavior_promise_ids = behavior_promises
        .iter()
        .map(|item| behavior_promise_id(item))
        .collect();
    let boundaries = boundaries_for(intent);
    let mut evidence = evidence_for(session);
    if scaffold_only {
        evidence.insert(
            0,
            format!(
                "Heads up: the implementation under `src/` is still a scaffold ({} module{}). Ask Claude Code to fill it in.",
                stub_paths.len(),
                if stub_paths.len() == 1 { "" } else { "s" }
            ),
        );
    }
    let run_invocation = run_invocation_for(intent);
    let followups = derive_followups(intent, session);
    Some(PlainEnglishSummary {
        schema_version: "x07.studio.plain_english_summary@0.1.0".to_string(),
        headline,
        behavior_promises,
        behavior_promise_ids,
        boundaries,
        evidence,
        run_invocation,
        followups,
        scaffold_only,
        stub_paths,
    })
}

pub fn behavior_promise_id(text: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
        if out.len() >= 64 {
            break;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "behavior".to_string()
    } else {
        out
    }
}

/// Walk the workspace's `src/` directory and flag every `*.x07.json`
/// whose `defn` bodies look like xtal-impl-sync stubs (single-literal
/// bodies, a `begin` block with one statement, etc.). Returns the
/// workspace-relative paths.
pub fn scan_stub_modules(root: &Utf8Path) -> Vec<String> {
    let mut paths = Vec::new();
    let src = root.join("src");
    if !src.exists() {
        return paths;
    }
    walk_x07_modules(src.as_std_path(), &src, &mut paths);
    paths.sort();
    paths
}

fn walk_x07_modules(dir: &std::path::Path, src_root: &Utf8Path, out: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_x07_modules(&path, src_root, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".x07.json"))
            .unwrap_or(false)
        {
            continue;
        }
        if !module_is_stub(&path) {
            continue;
        }
        let base = src_root
            .parent()
            .map(|p| p.as_std_path())
            .unwrap_or(src_root.as_std_path());
        let rel = path
            .strip_prefix(base)
            .ok()
            .and_then(|rel| rel.to_str())
            .map(|rel| rel.to_string())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        out.push(rel);
    }
}

fn module_is_stub(path: &std::path::Path) -> bool {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let decls = match value.get("decls").and_then(|v| v.as_array()) {
        Some(decls) => decls,
        None => return false,
    };
    let mut any_defn = false;
    let mut all_stubs = true;
    for decl in decls {
        if decl.get("kind").and_then(|v| v.as_str()) != Some("defn") {
            continue;
        }
        any_defn = true;
        let body = match decl.get("body") {
            Some(body) => body,
            None => continue,
        };
        if !body_is_stub(body) {
            all_stubs = false;
        }
    }
    any_defn && all_stubs
}

fn body_is_stub(body: &serde_json::Value) -> bool {
    // A `defn` body in x07AST is a single expression. The xtal-impl-sync
    // stubs emit shapes like ["bytes.empty"], ["i32.lit", 0],
    // ["bytes.lit", "todo"], or single-line begin blocks. Real impls
    // contain multiple nested begin/let/for/if operations.
    let arr = match body.as_array() {
        Some(arr) => arr,
        None => return true,
    };
    if arr.is_empty() {
        return true;
    }
    let head = arr.first().and_then(|v| v.as_str()).unwrap_or("");
    if matches!(
        head,
        "bytes.empty" | "bytes.lit" | "i32.lit" | "u32.lit" | "i64.lit" | "u64.lit" | "f64.lit"
    ) && arr.len() <= 3
    {
        return true;
    }
    if head == "begin" {
        // A "begin" with fewer than 4 children (begin + 0..2 statements +
        // tail) is still a stub. Real impls have several statements.
        let statement_count = arr.len().saturating_sub(1);
        if statement_count <= 2 {
            return true;
        }
        // Even with several statements, if every nested element is a
        // trivial literal we treat it as a stub.
        if arr.iter().skip(1).all(value_is_trivial_literal) {
            return true;
        }
    }
    // Heuristic floor on overall complexity: real impls usually serialize
    // to more than ~150 bytes of compact JSON. Skeletons sit well below.
    let serialized = serde_json::to_string(body).unwrap_or_default();
    serialized.len() < 80
}

fn value_is_trivial_literal(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Array(arr) => match arr.first().and_then(|v| v.as_str()) {
            Some("bytes.empty" | "bytes.lit" | "i32.lit" | "u32.lit" | "i64.lit" | "u64.lit") => {
                arr.len() <= 3
            }
            _ => false,
        },
        serde_json::Value::Number(_)
        | serde_json::Value::String(_)
        | serde_json::Value::Bool(_) => true,
        _ => false,
    }
}

/// Doctrine strings that `intent_packet_from_raw` seeds on every intent so
/// agents can see canonical x07/XTAL boundaries. They are useful to the
/// agent but feel like jargon to a non-engineer reading the summary, so the
/// summarizer hides them.
const DOCTRINE_FRAGMENTS: &[&str] = &[
    "canonical x07/xtal",
    "canonical x07 xtal",
    "spec-first xtal",
    "spec-changing repairs",
    "solve worlds deterministic",
    "do not turn the prompt directly into unchecked source code",
    "use the provided x07 spec as the canonical behavioral source",
    "os worlds, network, budget, and trust widening require explicit review",
    "rr fixtures, sandbox policy",
    "agent may edit implementation paths after spec approval",
    "agent may not widen specs or architecture policy without approval",
    "generated outputs, arch contracts, and budget profiles require drift evidence",
    // The seeded "examples" + "ambiguities" entries below are scaffold
    // placeholders, not real domain content. They were leaking into the
    // run-from-terminal snippet (as fake input) and the "Keep going"
    // suggestions (as nonsensical follow-ups). Treat them as doctrine.
    "input examples become spec examples before implementation",
    "generated tests must be reviewable before verify",
    "acceptance examples need final human approval",
    "proof strictness should be selected before certify",
];

pub(crate) fn is_doctrine(text: &str) -> bool {
    let lowered = text.trim().to_ascii_lowercase();
    DOCTRINE_FRAGMENTS
        .iter()
        .any(|fragment| lowered.contains(fragment))
}

fn headline_for(session: &SessionSnapshot, intent: &IntentPacket, scaffold_only: bool) -> String {
    let user_intent = first_desired_witness(intent)
        .map(short_summary)
        .or_else(|| short_summary_option(intent_source_text(intent)));
    let verb = match intent.task_type {
        TaskType::NewBehavior => "Built",
        TaskType::BugFix => "Fixed",
        TaskType::BehaviorChange => "Updated",
        TaskType::IncidentRepair => "Repaired",
        TaskType::BrownfieldExtract => "Extracted",
        TaskType::Explanation => "Explained",
    };
    // When the on-disk impl is still a stub we explicitly say so. Saying
    // "verified" for a scaffold misled users who tried to Try-It and got
    // empty output.
    let outcome = if scaffold_only {
        "— scaffolded; needs implementation."
    } else {
        match session.phase {
            SessionPhase::TrustReview | SessionPhase::CertifyRunning | SessionPhase::Certified => {
                "and verified."
            }
            SessionPhase::RepairEligible | SessionPhase::HumanInterventionRequired => {
                "— but it still needs your help."
            }
            _ => "and reviewed.",
        }
    };
    match user_intent {
        Some(text) => format!("{verb}: {text} {outcome}"),
        None => format!("{verb} your project {outcome}"),
    }
}

fn first_desired_witness(intent: &IntentPacket) -> Option<&str> {
    intent
        .witnesses
        .iter()
        .find(|witness| {
            matches!(witness.kind, WitnessKind::DesiredBehavior)
                && !is_doctrine(&witness.text)
                && !witness.text.trim().is_empty()
        })
        .map(|witness| witness.text.as_str())
}

fn intent_source_text(intent: &IntentPacket) -> Option<&str> {
    use loom_types::artifacts::IntentSource;
    match &intent.source {
        IntentSource::Text { raw } | IntentSource::Spec { raw } => Some(raw.as_str()),
        IntentSource::Voice { transcript } => Some(transcript.as_str()),
        IntentSource::Incident { path } => Some(path.as_str()),
        IntentSource::Sketch { path } => Some(path.as_str()),
        IntentSource::Image { path, .. } => Some(path.as_str()),
    }
}

fn short_summary_option(value: Option<&str>) -> Option<String> {
    value.map(short_summary)
}

fn short_summary(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let first_sentence = trimmed
        .split_terminator(['.', '!', '?', '\n'])
        .next()
        .unwrap_or(trimmed)
        .trim();
    let mut out = first_sentence.to_string();
    const MAX: usize = 120;
    if out.chars().count() > MAX {
        out = out.chars().take(MAX).collect::<String>();
        out.push('…');
    }
    out
}

fn behavior_promises_for(intent: &IntentPacket) -> Vec<String> {
    let mut out = Vec::new();
    for witness in &intent.witnesses {
        if !matches!(witness.kind, WitnessKind::DesiredBehavior) {
            continue;
        }
        let text = witness.text.trim();
        if text.is_empty() || is_doctrine(text) {
            continue;
        }
        out.push(text.to_string());
        if out.len() >= 5 {
            break;
        }
    }
    if out.is_empty() {
        for example in intent.examples.iter().take(3) {
            let text = example.trim();
            if text.is_empty() || is_doctrine(text) {
                continue;
            }
            out.push(text.to_string());
        }
    }
    out
}

fn boundaries_for(intent: &IntentPacket) -> Vec<String> {
    let mut out = Vec::new();
    for witness in &intent.witnesses {
        if !matches!(
            witness.kind,
            WitnessKind::ForbiddenBehavior | WitnessKind::PolicyRequirement
        ) {
            continue;
        }
        let text = witness.text.trim();
        if text.is_empty() || is_doctrine(text) {
            continue;
        }
        let line = match witness.kind {
            WitnessKind::ForbiddenBehavior => format!("Will not: {text}"),
            WitnessKind::PolicyRequirement => format!("Promises: {text}"),
            _ => text.to_string(),
        };
        out.push(line);
        if out.len() >= 4 {
            break;
        }
    }
    for policy in &intent.policy_implications {
        if out.len() >= 6 {
            break;
        }
        let text = policy.trim();
        if text.is_empty() || is_doctrine(text) {
            continue;
        }
        out.push(format!("Policy: {text}"));
    }
    out
}

fn evidence_for(session: &SessionSnapshot) -> Vec<String> {
    let mut out = Vec::new();
    let mut verify_passes = 0u32;
    let mut repair_rounds = 0u32;
    let mut tests_generated = false;
    let mut impl_synced = false;
    for op in &session.op_log {
        if op.op == "xtal.verify" {
            if matches!(op.status, loom_types::artifacts::OperationStatus::Succeeded) {
                verify_passes += 1;
            }
        } else if op.op == "xtal.repair" {
            repair_rounds += 1;
        } else if op.op == "tests.gen.write" {
            tests_generated = true;
        } else if op.op == "impl.sync.write" {
            impl_synced = true;
        }
    }
    if impl_synced {
        out.push("Wrote the implementation under approved write roots.".to_string());
    }
    if tests_generated {
        out.push("Generated tests from the approved examples.".to_string());
    }
    if verify_passes > 0 {
        out.push(format!(
            "Verified correctness ({} pass{}).",
            verify_passes,
            if verify_passes == 1 { "" } else { "es" }
        ));
    }
    if repair_rounds > 0 {
        out.push(format!(
            "Repaired {} time{} during build.",
            repair_rounds,
            if repair_rounds == 1 { "" } else { "s" }
        ));
    }
    if out.is_empty() {
        out.push(
            "Build pipeline ran end-to-end; see the detailed worklog for binding evidence."
                .to_string(),
        );
    }
    out
}

fn run_invocation_for(intent: &IntentPacket) -> Option<String> {
    if intent.targets.is_empty() {
        return None;
    }
    let example = intent
        .examples
        .iter()
        .find_map(|example| invocation_input_from_example(example))
        .unwrap_or_else(|| kind_aware_placeholder(intent));
    let escaped = example.replace('\\', "\\\\").replace('"', "\\\"");
    Some(format!(
        "printf \"%s\" \"{escaped}\" | x07 run --project x07.json --profile sandbox --stdin"
    ))
}

fn invocation_input_from_example(example: &str) -> Option<String> {
    let text = example.trim();
    if text.is_empty() || is_doctrine(text) {
        return None;
    }
    let candidate = text.split("->").next().unwrap_or(text).trim();
    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_string())
    }
}

/// Pick a plausible placeholder input based on the target module name so
/// the "Run it from your terminal" snippet is copy-pasteable as-is.
/// Falls back to a generic <your input here> when nothing matches.
fn kind_aware_placeholder(intent: &IntentPacket) -> String {
    let target = intent
        .targets
        .first()
        .map(|target| {
            format!(
                "{} {}",
                target.module_id,
                target.entry.as_deref().unwrap_or_default()
            )
        })
        .unwrap_or_default()
        .to_ascii_lowercase();
    if target.contains("sort") {
        "3 1 2 1 4".to_string()
    } else if target.contains("crawl") {
        "https://example.com".to_string()
    } else if target.contains("greet") {
        "World".to_string()
    } else if target.contains("calc") {
        "2 + 2".to_string()
    } else if target.contains("parse") || target.contains("validator") {
        "{\\\"hello\\\":\\\"world\\\"}".to_string()
    } else if target.contains("gateway") || target.contains("service") {
        "GET /health".to_string()
    } else if target.contains("workflow") || target.contains("graph") {
        "a:3 b:2 c:1 / a->b a->c".to_string()
    } else if target.contains("incident") || target.contains("guard") {
        "target/xtal/violations/<incident-id>".to_string()
    } else {
        "<your input here>".to_string()
    }
}

pub fn derive_followups(intent: &IntentPacket, _session: &SessionSnapshot) -> Vec<String> {
    // Heuristics first: prompts derived from the target module are almost
    // always more useful than generic XTAL ambiguities. Domain follow-ups
    // anchor the user; ambiguities tail in as a safety net only when we
    // run out of better suggestions.
    let mut out = Vec::new();
    let target = intent
        .targets
        .first()
        .map(|target| {
            format!(
                "{} {}",
                target.module_id,
                target.entry.as_deref().unwrap_or_default()
            )
        })
        .unwrap_or_default()
        .to_ascii_lowercase();
    if target.contains("sort") {
        push_followup(&mut out, "What about negative numbers?".to_string());
        push_followup(
            &mut out,
            "Should duplicate values preserve their input order?".to_string(),
        );
    } else if target.contains("crawl") {
        push_followup(&mut out, "What should it do for redirects?".to_string());
        push_followup(&mut out, "Should failed pages be retried?".to_string());
        push_followup(
            &mut out,
            "Should it respect robots.txt and a polite delay?".to_string(),
        );
    } else if target.contains("greet") {
        push_followup(
            &mut out,
            "Should it support multiple languages?".to_string(),
        );
        push_followup(
            &mut out,
            "Should it reject names with control characters?".to_string(),
        );
    } else if target.contains("calc") {
        push_followup(&mut out, "Should it support parentheses?".to_string());
        push_followup(&mut out, "What about division by zero?".to_string());
    } else if target.contains("parse") {
        push_followup(
            &mut out,
            "Should malformed input emit a structured error?".to_string(),
        );
    } else if target.contains("validator") {
        push_followup(
            &mut out,
            "Should errors include the failing JSON pointer?".to_string(),
        );
    } else if target.contains("gateway") {
        push_followup(
            &mut out,
            "Should unmatched routes return a structured error?".to_string(),
        );
    } else if target.contains("service") {
        push_followup(&mut out, "Should it expose a /health endpoint?".to_string());
        push_followup(&mut out, "What's the rate limit?".to_string());
    } else if target.contains("workflow") || target.contains("graph") {
        push_followup(
            &mut out,
            "What if two tasks have no edges between them?".to_string(),
        );
        push_followup(&mut out, "Should retries widen the makespan?".to_string());
    } else if target.contains("incident") {
        push_followup(
            &mut out,
            "Should this incident become a regression test?".to_string(),
        );
    }
    push_followup(&mut out, "Do you want a CLI wrapper for this?".to_string());
    push_followup(&mut out, "Should this become a service?".to_string());
    for ambiguity in &intent.ambiguities {
        let text = ambiguity.trim();
        if text.is_empty() || is_doctrine(text) {
            continue;
        }
        push_followup(&mut out, format!("What if {text}?"));
    }
    out.truncate(3);
    out
}

fn push_followup(target: &mut Vec<String>, value: String) {
    let text = value.trim();
    if text.is_empty() || is_doctrine(text) {
        return;
    }
    let normalized = text.to_string();
    if !target
        .iter()
        .any(|item| item.eq_ignore_ascii_case(&normalized))
    {
        target.push(normalized);
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use loom_types::artifacts::{IntentPacket, TaskType};
    use loom_types::session::{SessionPhase, SessionSnapshot};

    use super::{derive_followups, plain_english_summary_from_session};

    #[test]
    fn summary_uses_kind_aware_placeholder_when_no_examples() {
        // The demo IntentPacket targets `toy.sorter/sort_u8_asc`, so the
        // kind-aware placeholder should pick the sort-style example
        // instead of the generic <your input here>.
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "sorter", "/workspace", TaskType::NewBehavior);
        let mut intent = IntentPacket::demo(session_id, "/workspace");
        intent.examples.clear();
        session.intent = Some(intent);
        session.phase = SessionPhase::TrustReview;

        let summary = plain_english_summary_from_session(&session).expect("summary");

        assert_eq!(
            summary.run_invocation.as_deref(),
            Some("printf \"%s\" \"3 1 2 1 4\" | x07 run --project x07.json --profile sandbox --stdin")
        );
    }

    #[test]
    fn summary_falls_back_to_generic_placeholder_when_target_unknown() {
        // A session whose target doesn't match any kind heuristic still
        // produces a copy-pasteable command — just with a placeholder.
        let session_id = Uuid::new_v4();
        let mut session =
            SessionSnapshot::new(session_id, "demo", "/workspace", TaskType::NewBehavior);
        let mut intent = IntentPacket::demo(session_id, "/workspace");
        intent.examples.clear();
        intent.targets = vec![loom_types::artifacts::IntentTarget {
            module_id: "studio.example.unknown".to_string(),
            entry: Some("run_v1".to_string()),
        }];
        session.intent = Some(intent);
        session.phase = SessionPhase::TrustReview;

        let summary = plain_english_summary_from_session(&session).expect("summary");

        assert_eq!(
            summary.run_invocation.as_deref(),
            Some("printf \"%s\" \"<your input here>\" | x07 run --project x07.json --profile sandbox --stdin")
        );
    }

    #[test]
    fn derive_followups_leads_with_domain_heuristics_not_ambiguities() {
        // Heuristics first: for a sorter target the first suggestion should
        // be domain-specific, not an "What if …?" rephrasing of the
        // ambiguity list. Ambiguities still get a chance to fill the tail.
        let session_id = Uuid::new_v4();
        let session =
            SessionSnapshot::new(session_id, "sorter", "/workspace", TaskType::NewBehavior);
        let mut intent = IntentPacket::demo(session_id, "/workspace");
        intent.ambiguities = vec![
            "empty input should be rejected or accepted".to_string(),
            "sort order for equal values".to_string(),
            "numeric width".to_string(),
        ];

        let followups = derive_followups(&intent, &session);

        assert_eq!(followups.len(), 3);
        assert!(!followups[0].starts_with("What if"));
        assert!(followups
            .iter()
            .any(|item| item.contains("negative numbers")));
    }

    #[test]
    fn derive_followups_uses_generic_fallback() {
        let session_id = Uuid::new_v4();
        let session = SessionSnapshot::new(session_id, "demo", "/workspace", TaskType::NewBehavior);
        let mut intent = IntentPacket::demo(session_id, "/workspace");
        intent.targets.clear();
        intent.ambiguities.clear();

        let followups = derive_followups(&intent, &session);

        assert!(followups.iter().any(|item| item.contains("CLI wrapper")));
    }
}
