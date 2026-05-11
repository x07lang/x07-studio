# v0.1 status

## Wired now

- real x07 / x07-wasm / x07lp command execution through `loom-adapters::x07_cli`
- machine-readable report capture for x07 and x07-wasm
- structured stdout/stderr capture for x07-platform bindings
- streaming stdout/stderr updates for supervised coding-agent commands
- semantic `agent.event.*` records derived from supervised-agent artifacts, diagnostics, writes, and approval/policy requests
- binding coverage for XTAL authoring, test generation, implementation sync, verify/repair/certify/ingest/improve, core checks, web-ui, device, workload, topology, deploy-plan, and selected platform query/control reads
- session doctrine surfacing for canonical x07 docs, MCP tools, allowed verbs, and handoff prompt context
- MCP HTTP transport with initialize + session header handling
- MCP stdio transport with newline-delimited JSON-RPC
- daemon-owned `intent.formalize` endpoint for written plans, voice transcripts, incident notes, revision notes, and visible intent operation records
- OpenAI-compatible provider probing through `/models`, `/responses`, and `/chat/completions`
- Axum daemon routes for sessions, bindings, providers, and MCP connections
- egui GUI shell and ratatui Forge shell over the daemon API

## Still intentionally thin in v0.1

- GUI exposes HTTP MCP first; stdio MCP is available through the daemon API already
- provider probing is bounded and capability-oriented, not a full benchmark suite
- session execution policy is enforced by the reducer and canonical binding catalog, not yet by a full path sandbox
- the v0.1 shells expose lifecycle controls, basic lineage graph projection, and artifact logs; voice/STT, richer graph overlays, and full visual patch review remain later UI layers
- no compile-time proof cache yet
- no voice/stt layer yet

## Validation done here

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- focused tests cover reducer transitions, unknown-session handling, binding catalog rendering, provider probing, MCP tool parsing, and filesystem persistence
