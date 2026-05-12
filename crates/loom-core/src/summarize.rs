//! Deterministic plain-English summarizer over an approved intent and the
//! latest verify evidence. Emits `summary.plain_english` content that the
//! Simple-Mode UI renders as a one-paragraph behavior promise plus a few
//! supporting bullets ("Verified on N example cases", "Rejects empty input",
//! etc.) for non-engineer review.
//!
//! No LLM call — the summary is derived from the intent packet, spec
//! operations, witness list, and a bounded read of
//! `target/xtal/verify/summary.json`.

use serde::Serialize;

use loom_types::artifacts::{IntentPacket, TaskType, WitnessKind};
use loom_types::session::{SessionPhase, SessionSnapshot};

#[derive(Debug, Clone, Serialize)]
pub struct PlainEnglishSummary {
    pub schema_version: String,
    pub headline: String,
    pub behavior_promises: Vec<String>,
    pub boundaries: Vec<String>,
    pub evidence: Vec<String>,
}

impl PlainEnglishSummary {
    pub fn from_session(session: &SessionSnapshot) -> Option<Self> {
        let intent = session.intent.as_ref()?;
        let headline = headline_for(session, intent);
        let behavior_promises = behavior_promises_for(intent);
        let boundaries = boundaries_for(intent);
        let evidence = evidence_for(session);
        Some(Self {
            schema_version: "x07.studio.plain_english_summary@0.1.0".to_string(),
            headline,
            behavior_promises,
            boundaries,
            evidence,
        })
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
];

fn is_doctrine(text: &str) -> bool {
    let lowered = text.trim().to_ascii_lowercase();
    DOCTRINE_FRAGMENTS
        .iter()
        .any(|fragment| lowered.contains(fragment))
}

fn headline_for(session: &SessionSnapshot, intent: &IntentPacket) -> String {
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
    let outcome = match session.phase {
        SessionPhase::TrustReview | SessionPhase::CertifyRunning | SessionPhase::Certified => {
            "and verified."
        }
        SessionPhase::RepairEligible | SessionPhase::HumanInterventionRequired => {
            "— but it still needs your help."
        }
        _ => "and reviewed.",
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
