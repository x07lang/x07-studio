# Security review

Last updated 2026-05-13 for the pre-production readiness pass.

## A8.1 AGENT.md write-back path

Verdict: **fixed**.

Studio only writes `AGENT.md`; there is no request field that can redirect the path. The backend now rejects empty bodies and bodies larger than 64 KiB before writing a temp file inside the workspace and renaming it into place. Existing prior-hash checking still prevents blind overwrite when the file changed on disk.

Evidence:

- `crates/loom-core/src/agent_contract.rs`
- `crates/loom-types/src/api.rs` documents the 64 KiB request cap.
- `agent_contract::tests::write_rejects_oversized_agent_md`

## A8.2 File upload path

Verdict: **fixed**.

Image witnesses are capped at 4 MiB at both the axum body limit and handler check. The daemon accepts only `image/png`, `image/jpeg`, `image/webp`, and `image/gif`; active formats such as `text/html`, `image/svg+xml`, and `application/x-*` are rejected. The Composer uses the same MIME allowlist and size cap before dispatching.

Evidence:

- `crates/loom-daemon/src/lib.rs`
- `web/src/lib/components/Composer.svelte`
- `web/src/lib/api.ts`
- `image_upload_mime_allowlist_rejects_active_content`

## A8.3 MCP tool transparency

Verdict: **accepted-with-note**.

MCP calls parsed from supervised agent streams remain typed timeline turns with server, tool, input, and output. The browser card now shows the server origin, tool name, and redacts common sensitive argument names before rendering payloads. Direct `/v1/mcp/{connection_id}/call` calls are API utility calls and return the result to the caller; they are not attached to a session op unless they come through a supervised agent stream.

Evidence:

- `crates/loom-core/src/kernel.rs`
- `web/src/lib/components/McpCallCard.svelte`

## A8.4 Subprocess argv hygiene

Verdict: **clean**.

The x07, x07-wasm, x07lp, MCP stdio, Claude, and Codex subprocess paths use `Command::new` plus `.args(...)`; no shell string interpolation is used for user-controlled command execution.

Evidence:

- `crates/loom-adapters/src/command_runner.rs`
- `crates/loom-adapters/src/x07_cli.rs`
- `crates/loom-adapters/src/mcp.rs`
- `crates/loom-core/src/synthesis.rs`

## A8.5 Workspace path injection

Verdict: **clean with existing guardrails**.

Runtime path entry points use `validate_relative_runtime_path` or binding-specific relative-path validation before joining user-controlled paths into the workspace. Agent write audits also filter internal Studio session-store files so bookkeeping cannot be counted as user implementation writes.

Evidence:

- `crates/loom-core/src/kernel.rs`
- `crates/loom-adapters/src/x07_cli.rs`

## A8.6 Subscription-only cost gate

Verdict: **clean**.

The repository keeps `scripts/check_no_metered_api.py` as the subscription-only cost gate, and CI runs it on every push/PR. The script delegates to the Rust scan test that fails on commercial API hosts or API-key-only variables in executable source.

Evidence:

- `scripts/check_no_metered_api.py`
- `.github/workflows/ci.yml`
