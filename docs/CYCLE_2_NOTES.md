# Cycle 2 Notes

Cycle 2 turns x07 Studio from a guided prompt flow into a lifecycle operating
surface. The implementation keeps one shared Loom kernel and exposes the same
state through the browser, native Studio shell, Forge shell, and daemon API.

## Implemented

- Unified Timeline shell with no Simple/Expert mode toggle.
- Typed session-turn projection from the daemon.
- Plain-English verified summaries with behavior promises, boundaries,
  evidence, runnable invocation, and follow-up prompts.
- Follow-up refinement from trust review, certified, and repair-eligible phases.
- Try-It invocation through the x07 CLI with text, file, base64, and argv input.
- Shipping ladder state and rung-climb operations.
- Incident scanning and repair from canonical Studio, XTAL, and x07-wasm
  incident surfaces.
- Intent quorum rounds, image witnesses, cassette entries, cassette branching,
  project Q&A, sync codes, local memory, and visual parse/emit endpoints.
- Genpack-aware agent handoff prompts for detected service archetypes, including
  local `x07 service genpack schema` and `grammar` output when available.
- Trust binding coverage for sandbox reports, profile checks, and profile
  certification.
- Browser tests and connected daemon tests for the Timeline path, Atlas
  workflow lane, incident repair, and genpack-aware handoff prompts.

## Intentional Limits

- Quorum rounds are deterministic local projections, not live parallel agent
  execution.
- Sync codes are in-memory for the running daemon.
- Local Studio memory is append-only JSONL under `~/.x07-studio`, not a hosted
  identity or cloud sync system.
- Cassette branching creates a sibling session from a selected entry but does
  not yet replay or truncate every persisted artifact below that point.
- Visual parse/emit is a normalized graph exchange layer; full visual editors
  are a later cycle.

## Validation Targets

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && npm run check && npm test && npm run build && npm run e2e && npm run e2e:connected
```
