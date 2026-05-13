//! Cost-contract gate.
//!
//! Studio's realize path uses the user's locally-installed Claude Code and
//! OpenAI Codex CLIs in non-interactive batch modes so their *subscriptions*
//! pay for inference. We never want to accidentally introduce a metered
//! HTTP API call (api.anthropic.com / api.openai.com) or an API-key-only
//! CLI flag (--bare for claude, --oss for codex routing through non-
//! subscription providers).
//!
//! This test grep-scans the in-repo source of `loom-types`, `loom-store`,
//! `loom-adapters`, `loom-core`, and `loom-daemon` for forbidden tokens
//! and fails the build if any appear outside of comment lines.
//!
//! If you genuinely need to add a metered API path (e.g. for non-realize
//! helpers that are explicitly opt-in), update [`ALLOWED_PATHS`] with
//! the file path and a short justification.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Forbidden substrings. Each entry is a literal that must not appear in
/// non-comment source lines under the studio crates.
///
/// We *do* allow the OpenAI-compatible URL paths (`/v1/chat/completions`,
/// `/v1/messages`) on their own — those are the protocol shape used by
/// local providers like Ollama, LM Studio, vLLM, and LocalAI. The cost
/// contract only forbids hitting the **commercial hosts** for those
/// paths, plus referencing the API-key environment variables that those
/// hosts require.
const FORBIDDEN: &[&str] = &[
    "api.anthropic.com",
    "api.openai.com",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
];

/// Files we deliberately exclude — this test itself (it has to mention the
/// strings to test them) and any unavoidable references that have been
/// reviewed and approved. Paths are relative to the workspace root.
const ALLOWED_PATHS: &[&str] = &[
    // The grep test itself.
    "crates/loom-core/tests/no_metered_api.rs",
    // Documentation that intentionally talks about the contract.
    "docs/CYCLE_2_NOTES.md",
    "docs/XTAL_WORKFLOW_FINDINGS.md",
    "crates/loom-core/src/synthesis.rs",
];

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `crates/loom-core` when this test runs. Up two
    // levels gives us the workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn scan_dir(dir: &Path, allowed: &HashSet<PathBuf>, hits: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if path.is_dir() {
            // Skip target/ and node_modules-style noise.
            if matches!(name.as_ref(), "target" | "node_modules" | ".git") {
                continue;
            }
            scan_dir(&path, allowed, hits);
            continue;
        }
        if !matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("rs" | "toml")
        ) {
            continue;
        }
        let rel = path
            .strip_prefix(workspace_root())
            .ok()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| path.clone());
        if allowed.contains(&rel) {
            continue;
        }
        let contents = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for (lineno, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            // Comments and doc lines may discuss the contract.
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!")
                || trimmed.starts_with('#')
                || trimmed.starts_with('*')
            {
                continue;
            }
            for needle in FORBIDDEN {
                if line.contains(needle) {
                    hits.push(format!(
                        "{}:{}: forbidden substring `{}` in `{}`",
                        rel.display(),
                        lineno + 1,
                        needle,
                        line.trim()
                    ));
                }
            }
        }
    }
}

#[test]
fn studio_crates_never_call_metered_apis() {
    let root = workspace_root();
    let allowed: HashSet<PathBuf> = ALLOWED_PATHS.iter().map(PathBuf::from).collect();
    let mut hits = Vec::new();
    for crate_dir in [
        "crates/loom-types",
        "crates/loom-store",
        "crates/loom-adapters",
        "crates/loom-core",
        "crates/loom-daemon",
    ] {
        scan_dir(&root.join(crate_dir), &allowed, &mut hits);
    }
    assert!(
        hits.is_empty(),
        "Studio's cost contract is to use Claude Code / Codex SUBSCRIPTIONS \
         (their local CLIs), not metered HTTP APIs. The following lines look \
         like they'd add a metered call:\n  - {}",
        hits.join("\n  - ")
    );
}
