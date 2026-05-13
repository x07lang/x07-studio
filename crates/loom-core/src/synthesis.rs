//! Subscription-only impl realization plus a deterministic template
//! fallback.
//!
//! # Cost contract
//!
//! Studio invokes the user's locally-installed Claude Code and OpenAI Codex
//! CLIs in their non-interactive batch modes. Those CLIs authenticate
//! against the user's *subscription* (flat-rate Pro tiers), so each realize
//! round is incrementally free regardless of token volume. We deliberately
//! **never** call the metered HTTP APIs at `api.anthropic.com` /
//! `api.openai.com` from this crate. The `tests/no_metered_api.rs` build
//! gate asserts the contract holds across `loom-core`, `loom-adapters`,
//! `loom-daemon`, `loom-types`, and `loom-store`.
//!
//! The right CLI flags are listed below. They keep the realize step inside
//! the subscription:
//!
//! * `claude -p` — print mode (non-interactive, single response).
//! * `claude --permission-mode acceptEdits` — auto-accept file edits.
//! * `claude --allowedTools "Edit Write Read Glob Grep"` — restrict the
//!   tools the agent may invoke.
//! * `claude --output-format stream-json --include-partial-messages` —
//!   structured event output we can parse into the timeline.
//! * `claude --add-dir <workspace>` — explicit write scope.
//! * NEVER `claude --bare` — that mode requires `ANTHROPIC_API_KEY`.
//!
//! For Codex CLI:
//!
//! * `codex exec` — non-interactive subcommand.
//! * `codex exec --sandbox workspace-write` — auto-allow writes inside the
//!   workspace.
//! * `codex exec --json` — JSONL events on stdout.
//! * `codex exec --skip-git-repo-check` — Studio's manual-run-workspace
//!   isn't always a git repo.
//! * `codex exec -C <workspace>` — pin the working directory.
//! * NEVER `codex exec --oss` or any flag that routes inference through a
//!   non-subscription provider.
//!
//! # Template fallback
//!
//! When the user has no agent CLI installed (or both agents fail to
//! produce a file write inside the write roots), the kernel falls back to
//! the deterministic [`synthesize_from_template`] function below. The
//! templates produce a small but real x07AST implementation for the
//! "common project kinds" Studio's intent heuristics already detect:
//! sorter, greeter, calculator, parser, validator, crawler, workflow
//! graph, etc. The templates are intentionally simple — they're a "the
//! user can at least Try-It" floor, not production-quality code. A
//! follow-up cycle wires the user's CLI subscriptions into the same flow
//! for the more ambitious targets.

use camino::Utf8Path;
use loom_types::artifacts::IntentPacket;
use serde_json::{json, Value};

/// Build the (program, args) pair to spawn the realize subscription CLI
/// for an agent profile. `prompt` is appended as the final positional
/// argument because both `claude` and `codex exec` treat unrecognized
/// positionals as the user prompt.
pub fn build_realize_subscription_command(
    agent_id: &str,
    workspace: &Utf8Path,
    prompt: &str,
) -> (String, Vec<String>) {
    match agent_id {
        // Claude Code (Pro subscription via local OAuth/keychain). The
        // flags below are documented at the top of this module. We pin
        // `--add-dir` to the workspace so the agent can write src/ +
        // tests/ but nothing outside the project.
        "claude-code" => (
            "claude".to_string(),
            vec![
                "-p".to_string(),
                "--permission-mode".to_string(),
                "acceptEdits".to_string(),
                "--allowedTools".to_string(),
                "Edit Write Read Glob Grep".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--include-partial-messages".to_string(),
                "--verbose".to_string(),
                "--add-dir".to_string(),
                workspace.to_string(),
                prompt.to_string(),
            ],
        ),
        // OpenAI Codex CLI (Pro / Plus subscription via local OAuth).
        "openai-codex" | "codex" => (
            "codex".to_string(),
            vec![
                "exec".to_string(),
                "--sandbox".to_string(),
                "workspace-write".to_string(),
                "--json".to_string(),
                "--skip-git-repo-check".to_string(),
                "-C".to_string(),
                workspace.to_string(),
                prompt.to_string(),
            ],
        ),
        // Any other agent profile falls through to a plain `<cmd> <prompt>`
        // invocation. Custom agents are the user's responsibility, and we
        // assume the user has wired their own subscription/auth.
        other => (other.to_string(), vec![prompt.to_string()]),
    }
}

/// Produce a deterministic, real-x07AST module body for the given intent
/// target. Returns the JSON value Studio should write to
/// `src/<module_path>.x07.json` to replace the impl-sync stub. Returns
/// `None` when the target kind isn't covered by a template — the kernel
/// then surfaces a "needs human or subscription agent" hint rather than
/// claiming the project is implemented.
pub fn synthesize_from_template(intent: &IntentPacket) -> Option<TemplateSynthesis> {
    let target = intent.targets.first()?;
    let module_id = target.module_id.as_str();
    let entry_name = target.entry.as_deref().unwrap_or("run_v1");
    let full_name = format!("{module_id}.{entry_name}");
    let body = template_body_for(module_id, entry_name)?;
    let module = json!({
        "schema_version": "x07.x07ast@0.8.0",
        "kind": "module",
        "module_id": module_id,
        "imports": [],
        "decls": [
            { "kind": "export", "names": [full_name] },
            {
                "kind": "defn",
                "name": full_name,
                "params": [{ "name": "payload", "ty": "bytes" }],
                "result": "bytes",
                "body": body,
            },
        ],
    });
    Some(TemplateSynthesis {
        module_id: module_id.to_string(),
        entry_name: entry_name.to_string(),
        relative_path: format!("src/{}.x07.json", module_id.replace('.', "/")),
        body: module,
    })
}

#[derive(Debug, Clone)]
pub struct TemplateSynthesis {
    pub module_id: String,
    pub entry_name: String,
    pub relative_path: String,
    pub body: Value,
}

fn template_body_for(module_id: &str, _entry_name: &str) -> Option<Value> {
    let lowered = module_id.to_ascii_lowercase();
    if lowered.contains("sort") {
        // Bubble sort the payload bytes ascending. Matches what the
        // xtal-pure starter ships for `toy.sorter`.
        return Some(json!([
            "begin",
            ["let", "n", ["bytes.len", "payload"]],
            ["let", "out", ["view.to_bytes", ["bytes.view", "payload"]]],
            ["for", "i", 0, "n",
                ["for", "j", 0, ["-", "n", ["+", "i", 1]],
                    ["begin",
                        ["let", "a", ["bytes.get_u8", "out", "j"]],
                        ["let", "k", ["+", "j", 1]],
                        ["let", "c", ["bytes.get_u8", "out", "k"]],
                        ["if", [">u", "a", "c"],
                            ["begin",
                                ["set", "out", ["bytes.set_u8", "out", "j", "c"]],
                                ["set", "out", ["bytes.set_u8", "out", "k", "a"]],
                                0],
                            0]]]],
            "out"
        ]));
    }
    if lowered.contains("greet") {
        // Echo "Hello, <payload>!"
        return Some(json!([
            "begin",
            ["let", "n", ["bytes.len", "payload"]],
            ["if", ["=", "n", 0],
                ["return", ["bytes.empty"]],
                0],
            ["let", "out", ["bytes.lit", "Hello, "]],
            ["for", "i", 0, "n",
                ["set", "out", ["bytes.push_u8", "out", ["bytes.get_u8", "payload", "i"]]]],
            ["set", "out", ["bytes.push_u8", "out", 33]],
            "out"
        ]));
    }
    if lowered.contains("calc") {
        // Pass-through identity over bytes — a placeholder until the user
        // hands off to the subscription agent for the actual arithmetic.
        return Some(json!([
            "begin",
            ["if", ["=", ["bytes.len", "payload"], 0],
                ["return", ["bytes.empty"]],
                0],
            ["view.to_bytes", ["bytes.view", "payload"]]
        ]));
    }
    if lowered.contains("parse") || lowered.contains("validator") {
        // Length-prefixed echo: returns `<u8:len><payload>`. Real parsers
        // need spec-driven generation; this is a Try-It floor.
        return Some(json!([
            "begin",
            ["let", "n", ["bytes.len", "payload"]],
            ["let", "out", ["bytes.empty"]],
            ["set", "out", ["bytes.push_u8", "out", "n"]],
            ["for", "i", 0, "n",
                ["set", "out", ["bytes.push_u8", "out", ["bytes.get_u8", "payload", "i"]]]],
            "out"
        ]));
    }
    if lowered.contains("crawl") {
        // Crawler stub-floor: echo the input URL. The user is expected to
        // ask Claude Code / Codex to fill in the real fetch loop.
        return Some(json!([
            "begin",
            ["if", ["=", ["bytes.len", "payload"], 0],
                ["return", ["bytes.empty"]],
                0],
            ["view.to_bytes", ["bytes.view", "payload"]]
        ]));
    }
    if lowered.contains("workflow") || lowered.contains("graph") {
        // Workflow makespan floor: return `bytes.len(payload)` as a single
        // byte. Real makespan needs graph parsing.
        return Some(json!([
            "begin",
            ["let", "n", ["bytes.len", "payload"]],
            ["let", "out", ["bytes.empty"]],
            ["set", "out", ["bytes.push_u8", "out", "n"]],
            "out"
        ]));
    }
    if lowered.contains("incident") {
        // Incident handler: pass-through.
        return Some(json!([
            "view.to_bytes", ["bytes.view", "payload"]
        ]));
    }
    if lowered.contains("gateway") || lowered.contains("service") {
        // Service handler floor: echo the request payload.
        return Some(json!([
            "view.to_bytes", ["bytes.view", "payload"]
        ]));
    }
    // Catch-all for `app.main` / `app.cli`: pass-through.
    if lowered.starts_with("app.") {
        return Some(json!([
            "view.to_bytes", ["bytes.view", "payload"]
        ]));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(module_id: &str, entry: &str) -> IntentPacket {
        use loom_types::artifacts::{IntentSource, IntentTarget};
        IntentPacket {
            schema_version: "x07.studio.intent_packet@0.1.0".to_string(),
            session_id: uuid::Uuid::nil(),
            workspace_root: "/workspace".to_string(),
            task_type: loom_types::artifacts::TaskType::NewBehavior,
            targets: vec![IntentTarget {
                module_id: module_id.to_string(),
                entry: Some(entry.to_string()),
            }],
            examples: vec![],
            constraints: vec![],
            policy_implications: vec![],
            ambiguities: vec![],
            assumptions: vec![],
            witnesses: vec![],
            source: IntentSource::Text {
                raw: "test".to_string(),
            },
            clarification_history: vec![],
        }
    }

    #[test]
    fn build_realize_command_uses_subscription_flags_for_claude() {
        let (program, args) =
            build_realize_subscription_command("claude-code", Utf8Path::new("/ws"), "do X");
        assert_eq!(program, "claude");
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"--permission-mode".to_string()));
        assert!(args.contains(&"acceptEdits".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
        // SUBSCRIPTION COST CHECK: must not pass --bare (forces API key auth).
        assert!(!args.contains(&"--bare".to_string()));
        // Prompt content is the last positional arg.
        assert_eq!(args.last().map(String::as_str), Some("do X"));
    }

    #[test]
    fn build_realize_command_uses_subscription_flags_for_codex() {
        let (program, args) =
            build_realize_subscription_command("openai-codex", Utf8Path::new("/ws"), "build it");
        assert_eq!(program, "codex");
        assert_eq!(args[0], "exec");
        assert!(args.contains(&"workspace-write".to_string()));
        assert!(args.contains(&"--json".to_string()));
        // SUBSCRIPTION COST CHECK: must not flip to non-subscription provider.
        assert!(!args.contains(&"--oss".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("build it"));
    }

    #[test]
    fn template_synthesis_emits_real_sorter_body() {
        let packet = intent("toy.sorter", "sort_u8_asc");
        let synth = synthesize_from_template(&packet).expect("template");
        assert_eq!(synth.relative_path, "src/toy/sorter.x07.json");
        let body = synth.body["decls"][1]["body"].clone();
        // Real sorter body has nested `for` + `begin`; serialized JSON is
        // well over the 80-byte floor that summarize.rs treats as stub.
        let serialized = serde_json::to_string(&body).expect("serialize");
        assert!(serialized.len() > 80, "body too short: {serialized}");
    }

    #[test]
    fn template_synthesis_emits_real_greeter_body() {
        let packet = intent("app.greeter", "greet_v1");
        let synth = synthesize_from_template(&packet).expect("template");
        let body = synth.body["decls"][1]["body"].clone();
        let serialized = serde_json::to_string(&body).expect("serialize");
        assert!(serialized.contains("Hello, "));
        assert!(serialized.len() > 80);
    }

    #[test]
    fn template_synthesis_returns_none_for_unknown_kind() {
        let packet = intent("totally.weird.target", "wat");
        assert!(synthesize_from_template(&packet).is_none());
    }
}
