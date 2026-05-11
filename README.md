# x07-studio

Rust-native v0.1 workspace for the x07 Studio system:

- **Loom**: lifecycle kernel, adapters, store, REST daemon, and client.
- **x07 Studio**: desktop shell built with `eframe` / `egui`.
- **Forge**: terminal shell built with `ratatui`.
- **Studio Web**: browser shell built with SvelteKit over the Loom REST API.

This repo keeps the x07/XTAL lifecycle artifact-centric:

```text
intent -> spec -> realization -> verify -> repair -> trust/certify -> ingest -> improve
```

## What v0.1 wires for real

- **Real x07 CLI execution** using the documented machine-output contract (`--json`, `--report-out`, `--quiet-json`) through the canonical x07 / x07-wasm / x07lp CLIs.
- **Binding-first lifecycle commands** for XTAL authoring, test generation, implementation sync, verify/repair/certify, incident ingest/improve, x07 core checks, x07-wasm web/device/workload/deploy lanes, and selected platform query/control reads.
- **Real MCP transport** for both:
  - HTTP JSON-RPC over `/mcp` with `initialize`, `notifications/initialized`, session headers, and `tools/list` / `tools/call`
  - stdio MCP with newline-delimited JSON-RPC
- **Live provider probing** for OpenAI-compatible endpoints:
  - `GET /models`
  - optional deep probes against `/responses`
  - optional deep probes against `/chat/completions`

## Workspace layout

```text
x07-studio/
├── AGENT.md
├── .agent/skills/x07-studio-flow/SKILL.md
├── config/
│   ├── providers.example.json
│   ├── mcp-http.example.json
│   └── mcp-stdio.example.json
├── docs/
│   ├── ARCHITECTURE.md
│   ├── API.md
│   ├── XTAL_WORKFLOW_FINDINGS.md
│   └── COMMAND_BINDINGS.md
├── web/
│   ├── src/
│   └── static/mockups/x07-studio-xtal-ui-mockup.png
├── schemas/
│   ├── index.json
│   ├── x07.studio.intent_packet.schema.json
│   ├── x07.studio.lineage_graph.schema.json
│   ├── x07.studio.op_record.schema.json
│   ├── x07.studio.provider_probe_report.schema.json
│   ├── x07.studio.provider_profile.schema.json
│   ├── x07.studio.session_contract.schema.json
│   └── x07.studio.session_snapshot.schema.json
└── crates/
    ├── loom-types/
    ├── loom-core/
    ├── loom-store/
    ├── loom-adapters/
    ├── loom-client/
    ├── loom-daemon/
    ├── x07-studio/
    └── x07-studio-forge/
```

## Toolchain

- Rust `1.92.0` is pinned in `rust-toolchain.toml`.
- `cargo` will pick that toolchain automatically if `rustup` is installed.

## Quickstart

```bash
cargo run -p loom-daemon -- serve --root /path/to/x07/workspace --addr 127.0.0.1:7719
cargo run -p x07-studio -- --daemon-url http://127.0.0.1:7719
cargo run -p x07-studio-forge -- --daemon-url http://127.0.0.1:7719
cd web && npm install && npm run dev
```

## Suggested local setup

1. Put this repo beside `x07`, `x07-mcp`, `x07-wasm-backend`, and `x07-platform`.
2. Make sure the canonical CLIs are on `PATH`, or set overrides:
   - `X07_STUDIO_X07_EXE`
   - `X07_STUDIO_X07_WASM_EXE`
   - `X07_STUDIO_X07LP_EXE`
3. Copy `config/providers.example.json` for provider setup and `config/mcp-http.example.json` or `config/mcp-stdio.example.json` for MCP connection payloads.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && npm run check && npm test && npm run build
```

## Notes

- The daemon is the operational center in v0.1. The GUI and TUI are deliberately thin clients over the daemon REST surface.
- MCP connectivity is stateful. The daemon keeps open HTTP/stdio MCP sessions in memory while it is running.
- Provider probing is intentionally low-cost. Deep probes are optional and bounded.
- Studio is a sibling ecosystem repo. X07 language semantics, XTAL artifact schemas, package management, and canonical docs stay in `x07`.
