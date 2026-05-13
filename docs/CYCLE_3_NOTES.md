# Cycle 3 Notes

Cycle 3 makes Studio less button-driven and more transparent while keeping the
same Loom kernel and XTAL lifecycle.

## Implemented

- Live Claude Code `stream-json` and Codex `--json` normalization into
  `AgentStreamEvent` records on the existing session SSE stream.
- Live diff artifacts under `.x07/studio/diffs/` for streamed Edit / Write
  tool-use events, plus `/diffs/live` SSE for diff-only consumers.
- Realize quorum for Claude Code and Codex using private staging workspaces,
  digest comparison, side-by-side web review, and proposal pick/apply.
- Autopilot planner and daemon loop for high-confidence clarify defaults,
  spec approval, build, optional realize, and optional ladder climb.
- Voice-first composer path with Web Speech transcript confidence and daemon
  persistence on the `intent.formalize` op, including a direct `/intent/voice`
  endpoint for host shells that already have a transcript.
- Persistent sync code state blobs for cross-device session continuation.
- Memory preference application at session creation, surfaced as
  `preferences.apply` evidence and editable in the web drawer.
- Release submission over existing x07lp deploy bindings and pollable release
  status records.
- Replay capsule export/import as local deterministic JSON capsules with a
  manifest digest.
- Visual canvas primitives for pan/zoom, nodes, and edges over the existing
  stream-pipe, state-machine, and task-DAG parse/emit endpoints.
- Connected fake toolchain now writes impl-sync stubs to the intent-derived
  module path and emits realistic stream events during fake realize.
- The incident watch endpoint starts a bounded background watcher and the web
  composer shows an incident-arrived badge when new incident turns stream in.

## Cost Contract

Cycle 3 keeps the Cycle 2 subscription-only contract. Studio still runs local
Claude Code and Codex CLIs in batch modes and does not add metered provider API
routes. The `no_metered_api` Rust test remains the guard.

## Validation Targets

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && npm run check && npm test && npm run build && npm run e2e && npm run e2e:connected
```
