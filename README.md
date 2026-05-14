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
- **Unified Timeline browser surface** over typed session turns, plain-English verified summaries, runnable `x07 run` invocations, follow-up refinements, Try-It execution, shipping ladder state, trust posture, semantic diffs, proof evidence, incident quickfixes, cassette replay branches, persistent sync codes with state blobs, local memory, live agent stream and MCP call events, realize quorum comparison, autopilot, release submission, certificate review, replay export, and visual canvas editors.
- **Process Lane + role collaboration** over the same session op log: canonical x07 steps, live actor attribution, current/next forecast, per-step evidence drilldown, Architect/Coder/Reviewer routing, role overrides, self-review fallback, step budgets, and manual Second opinion quorum.
- **Canonical x07 agent loop surfaces** for `AGENT.md`, `x07 lint`/`x07 fix`, `x07 doctor`, `x07 pkg lock --check`, migration checks, `x07 test --pbt`, `x07 arch check`, `x07 pkg provides`, and the ten official agent-gate recipes.
- **Genpack-aware agent handoffs** that embed local x07 service archetype schema and grammar for service-shaped intents before Codex or Claude drafts artifacts.
- **Pre-production stress coverage** for the real toolchain scenarios: CSV repair pause, run-os/os-time trust widening, PBT counterexample regression capture, and Architect/Coder/Reviewer role-pipeline review acceptance. See `docs/PRODUCTION_READINESS.md` and `docs/STRESS_PASS_FINDINGS.md`.
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
│   ├── INDEX.md
│   ├── USER_GUIDE.md
│   ├── AGENT_GUIDE.md
│   ├── TROUBLESHOOTING.md
│   ├── ACCESSIBILITY.md
│   ├── TELEMETRY.md
│   ├── PERFORMANCE.md
│   ├── OBSERVABILITY.md
│   ├── SECURITY_REVIEW.md
│   ├── PRODUCTION_READINESS.md
│   ├── API.md
│   ├── TIMELINE_MODE.md
│   ├── XTAL_WORKFLOW_FINDINGS.md
│   ├── COMMAND_BINDINGS.md
│   ├── CYCLE_2_NOTES.md
│   ├── CYCLE_3_NOTES.md
│   ├── CYCLE_4_NOTES.md
│   ├── CYCLE_5_NOTES.md
│   ├── CYCLE_6_NOTES.md
│   ├── AGENT_CONTRACT.md
│   ├── AGENT_ROLES.md
│   ├── PROCESS_LANE.md
│   ├── WHAT_IF.md
│   ├── STEP_BUDGETS.md
│   ├── LINT_LOOP.md
│   ├── HEALTH_ROW.md
│   ├── PBT_FLOW.md
│   ├── CANONICAL_RECIPES.md
│   ├── TRUST_CARD.md
│   ├── SEMANTIC_DIFF.md
│   ├── PROOF_EXPLORER.md
│   ├── VISUAL_SYSTEM.md
│   ├── STREAM_EVENTS.md
│   ├── AUTOPILOT.md
│   └── design/xtal-studio-ui-mockup.png
├── web/
│   └── src/
├── schemas/
│   ├── index.json
│   ├── x07.studio.intent_packet.schema.json
│   ├── x07.studio.agent_contract.schema.json
│   ├── x07.studio.lint_report.schema.json
│   ├── x07.studio.health_snapshot.schema.json
│   ├── x07.studio.pbt_round.schema.json
│   ├── x07.studio.arch_check_report.schema.json
│   ├── x07.studio.pkg_provides_result.schema.json
│   ├── x07.studio.lineage_graph.schema.json
│   ├── x07.studio.op_record.schema.json
│   ├── x07.studio.trust_posture.schema.json
│   ├── x07.studio.semantic_diff.schema.json
│   ├── x07.studio.proof_evidence.schema.json
│   ├── x07.studio.quickfix_record.schema.json
│   ├── x07.studio.cassette_ribbon.schema.json
│   ├── x07.studio.canonical_step.schema.json
│   ├── x07.studio.process_lane.schema.json
│   ├── x07.studio.review_round.schema.json
│   ├── x07.studio.role_pipeline.schema.json
│   ├── x07.studio.role_preferences.schema.json
│   ├── x07.studio.step_budget.schema.json
│   ├── x07.studio.what_if_forecast.schema.json
│   ├── x07.studio.certificate_summary.schema.json
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

The native desktop shell can also start its own local daemon:

```bash
cargo run -p x07-studio -- --root /path/to/x07/workspace
```

The packaged web shell uses the same daemon API and static Svelte build:

```bash
cd web && npm install && npm run build
cargo build --release -p loom-daemon -p x07-studio -p x07-studio-forge
python3 scripts/package_standalone.py --target-dir target/release --web-dir web/build --out-dir dist/standalone
```

Inside a standalone bundle, `python3 scripts/launch_studio_web.py --bundle-root .`
is the preferred entry point. It detects bundled components, builds missing
sibling source checkouts when they are present, refreshes `defaults.env`, starts
the daemon, and opens the Svelte Studio surface with the same onboarding panel.
The native shell under `bin/x07-studio` runs the same packaged bootstrap first
unless `--skip-bootstrap` is passed, then reads the bundle `defaults.env` before
it starts its embedded daemon. Use `--no-install-missing` when the desktop shell
should only detect existing components. The generated defaults choose
`~/x07-studio-workspace`, local daemon/web addresses, and bundled component
paths when release automation includes them.

## Suggested local setup

1. Put this repo beside `x07`, `x07-mcp`, `x07-wasm-backend`, and `x07-platform`.
2. Make sure the canonical CLIs are on `PATH`, or set overrides:
   - `X07_STUDIO_X07_EXE`
   - `X07_STUDIO_X07_WASM_EXE`
   - `X07LP_BINARY` or `X07_STUDIO_X07LP_EXE`
3. Run `python3 scripts/bootstrap_components.py --install-missing --write-env .x07/studio/defaults.env` to detect available tools and build sibling source checkouts when possible.
4. Copy `config/providers.example.json` for provider setup and `config/mcp-http.example.json` or `config/mcp-stdio.example.json` for MCP connection payloads.

The Studio health endpoint and web onboarding panel report readiness for `x07`,
`x07-wasm`, `x07lp`, Codex, and Claude Code. The first three are required for
the full Atlas release and local platform delivery lane; the agent CLIs are
optional until supervised handoffs need to execute locally.

## Documentation

- Start with `docs/INDEX.md` for the audience-oriented doc map.
- Use `docs/USER_GUIDE.md` for local usage and first-session walkthrough.
- Use `docs/AGENT_GUIDE.md` for Codex/Claude handoff semantics, write roots, and MCP event visibility.
- Use `docs/TROUBLESHOOTING.md` for daemon, upload, AGENT.md, and trust-posture issues.
- Use `docs/PRODUCTION_READINESS.md` for public-beta and GA gates.
- Use `docs/SECURITY_REVIEW.md` for the latest pre-production security pass.
- Use `docs/TELEMETRY.md`, `docs/PERFORMANCE.md`, and `docs/OBSERVABILITY.md` for GA evidence collection and operator checks.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && npm run check && npm test && npm run build && npm run e2e && npm run e2e:connected
python3 scripts/check_no_metered_api.py
```

GitHub Actions builds the Rust workspace, the Svelte web app, Playwright E2E,
the connected web-to-daemon XTAL smoke path, and standalone desktop bundles for
Linux, macOS, and Windows 2022. The desktop bundle job also builds `x07-wasm`
from `x07lang/x07-wasm-backend` and wires it into the packaged defaults so
Atlas app workflows have a ready WASM component. CI validates each bundle's
manifest, web app, launcher scripts, first-run defaults, zip archive, and
bundled `x07-wasm` bootstrap status.

Additional pre-production harnesses:

```bash
python3 scripts/cross_browser_smoke.py
python3 scripts/stability_soak.py
python3 scripts/a11y_audit.py
python3 scripts/perf_budget.py
python3 scripts/slo_dashboard.py --root /path/to/workspace --addr http://127.0.0.1:7719
./scripts/test_linux.sh
python3 scripts/stress_pass.py init --scenario scenario-2-csv-repair
```

`stability_soak.py` defaults to the public-beta A7 workload: one formalized
session with repeated autopilot starts, health polling, RSS/FD metrics, and
op-log/subscriber-count tracking. Use `--workload build-loop` when you need the
older multi-session build-cycle stress pattern.

`stress_pass.py init --scenario scenario-2-csv-repair` seeds
`target/stress-pass/scenario-2-csv-repair/workspace` from the CSV repair
fixture when that workspace is missing. Boot the daemon against the seeded
workspace to exercise the real `xtal.verify` failure -> `xtal.repair` -> clean
`realize_stalled` pause path.

## Notes

- The daemon is the operational center in v0.1. The GUI and TUI are deliberately thin clients over the daemon REST surface.
- MCP connectivity is stateful. The daemon keeps open HTTP/stdio MCP sessions in memory while it is running.
- Provider probing is intentionally low-cost. Deep probes are optional and bounded.
- Studio is a sibling ecosystem repo. X07 language semantics, XTAL artifact schemas, package management, and canonical docs stay in `x07`.
- Cycle 4 UX surfaces are artifact-first. Trust cards, diffs, proof drawers, quickfixes, cassette ribbons, and certificates are projections over operation logs and canonical workspace files, not browser-only state.
- Cycle 5 surfaces speak the documented x07 agent loop directly: health, lint, fix, PBT, arch-check, package discovery, and `AGENT.md` are daemon-backed projections over canonical x07 commands.
- Cycle 6 makes the active process visible. The Process Lane is reproducible from the operation log, while role preferences live in `~/.x07-studio/memory.jsonl` and per-session overrides are recorded as session operations.
