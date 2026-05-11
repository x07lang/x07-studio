# v0.1 status

## Wired now

- real x07 / x07-wasm / x07lp command execution through `loom-adapters::x07_cli`
- machine-readable report capture for x07 and x07-wasm
- structured stdout/stderr capture for x07-platform bindings
- streaming stdout/stderr updates for supervised coding-agent commands
- semantic `agent.event.*` records derived from supervised-agent artifacts, diagnostics, writes, and approval/policy requests
- browser trust review queue derived from artifacts, diagnostics, writes, patchsets, verify evidence, and certify evidence
- visual patch review in the operation inspector for x07 patchset payloads, patchset artifacts, write roots, review gates, path risk, and before/after JSON previews
- world/budget guard for solve-rr, sandbox/run-os, WASM app, release/provenance, and budget widening surfaces
- bounded daemon artifact preview for recorded operation artifacts, including in-memory JSON Patch previews for x07 patchsets
- binding coverage for XTAL authoring, test generation, implementation sync, verify/repair/certify/ingest/improve, core checks, app, web-ui, device, workload, topology, deploy-plan, SLO, provenance, and selected platform query/control reads
- seeded docs-example workflows for workflow graph, state-machine contracts, API gateway, x07crawl, x07dbguard, and x07 Atlas projects
- approval ledger that blocks stale approvals after human revision requests until the agent repolishes intent
- session doctrine surfacing for canonical x07 docs, MCP tools, allowed verbs, and handoff prompt context
- agent handoff execution-boundary prompts for x07 run, solve-rr, sandbox/run-os, WASM app, release/provenance, and SLO/budget gates
- MCP HTTP transport with initialize + session header handling
- MCP stdio transport with newline-delimited JSON-RPC
- daemon-owned `intent.formalize` endpoint for written plans, voice transcripts, existing specs, incident notes, revision notes, and visible intent operation records
- OpenAI-compatible provider probing through `/models`, `/responses`, and `/chat/completions`
- Axum daemon routes for sessions, bindings, providers, and MCP connections
- egui GUI shell and ratatui Forge shell over the daemon API

## Still intentionally thin in v0.1

- GUI exposes HTTP MCP first; stdio MCP is available through the daemon API already
- provider probing is bounded and capability-oriented, not a full benchmark suite
- session execution policy is enforced by the reducer and canonical binding catalog, not yet by a full path sandbox
- the v0.1 shells expose lifecycle controls, basic lineage graph projection, artifact logs, a compact trust review queue, artifact-backed patchset previews, and path-level before/after visual patch review; voice/STT, richer graph overlays, and semantic side-by-side diff tooling remain later UI layers
- no compile-time proof cache yet
- no voice/stt layer yet

## Validation done here

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- focused tests cover reducer transitions, unknown-session handling, binding catalog rendering, provider probing, MCP tool parsing, and filesystem persistence
