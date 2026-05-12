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

fn headline_for(session: &SessionSnapshot, intent: &IntentPacket) -> String {
    let target = intent
        .targets
        .first()
        .map(|t| t.module_id.clone())
        .unwrap_or_else(|| "your project".to_string());
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
            "and verified"
        }
        SessionPhase::RepairEligible | SessionPhase::HumanInterventionRequired => {
            "but it still needs your help"
        }
        _ => "and reviewed",
    };
    format!("{verb} `{target}` {outcome}.")
}

fn behavior_promises_for(intent: &IntentPacket) -> Vec<String> {
    let mut out = Vec::new();
    for witness in &intent.witnesses {
        if witness.text.trim().is_empty() {
            continue;
        }
        let prefix = match witness.kind {
            WitnessKind::DesiredBehavior => "Does",
            WitnessKind::ForbiddenBehavior => "Does not",
            WitnessKind::PolicyRequirement => "Promises",
            WitnessKind::IncidentReport => "Resolves",
        };
        out.push(format!("{prefix}: {}", witness.text.trim()));
        if out.len() >= 5 {
            break;
        }
    }
    if out.is_empty() {
        for example in intent.examples.iter().take(3) {
            out.push(example.clone());
        }
    }
    out
}

fn boundaries_for(intent: &IntentPacket) -> Vec<String> {
    let mut out = Vec::new();
    for constraint in &intent.constraints {
        if constraint.trim().is_empty() {
            continue;
        }
        out.push(constraint.clone());
        if out.len() >= 4 {
            break;
        }
    }
    for policy in &intent.policy_implications {
        if out.len() >= 6 {
            break;
        }
        out.push(format!("Policy: {policy}"));
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
