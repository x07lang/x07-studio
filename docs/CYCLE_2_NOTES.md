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
- Live parallel intent quorum rounds, image witnesses, cassette entries,
  replaying cassette branches, project Q&A, persistent sync codes, local
  memory, visual parse/emit endpoints, and browser visual graph editors.
- Genpack-aware agent handoff prompts for detected service archetypes, including
  local `x07 service genpack schema` and `grammar` output when available.
- Trust binding coverage for sandbox reports, profile checks, and profile
  certification.
- Browser tests and connected daemon tests for the Timeline path, Atlas
  workflow lane, incident repair, genpack-aware handoff prompts, live quorum,
  sync claims, cassette branching, and visual graph editing.

## Intentional Limits

- Local Studio memory is append-only JSONL under `~/.x07-studio`, not a hosted
  identity or cloud sync system.
- Visual editors are local graph editors for the currently supported
  `streampipe`, `statemachine`, and `tasks` graph exchange layer; they are not a
  domain-specific layout engine for every future x07 surface.

## Subscription-Only Cost Contract

Studio's realize pipeline uses the user's **locally-installed Claude Code
and OpenAI Codex CLIs in their non-interactive batch modes**, so flat-rate
Pro subscriptions pay for inference. We never invoke metered HTTP APIs
(`api.anthropic.com`, `api.openai.com`) from this codebase, and we never
pass API-key-only CLI flags (`--bare` for `claude`, `--oss` for `codex`)
that would route inference through metered providers.

### How the flags translate

The realize handoff spawns either CLI with these flags:

| Need | Claude Code | Codex |
| --- | --- | --- |
| Non-interactive batch | `-p` | `exec` |
| Auto-accept file edits | `--permission-mode acceptEdits` | `--sandbox workspace-write` |
| Structured event output | `--output-format stream-json --include-partial-messages` | `--json` |
| Scoped write access | `--add-dir <workspace>` | `-C <workspace>` |
| Tool allowlist | `--allowedTools "Edit Write Read Glob Grep"` | (sandbox covers this) |
| Allow non-git directories | (n/a) | `--skip-git-repo-check` |
| Prompt input | last positional argv | last positional argv |

The full flag set lives in `crates/loom-core/src/synthesis.rs`
(`build_realize_subscription_command`) and is unit-tested with the
"flags assert no metered routes" pair in the same file.

### Template fallback (no CLI, no API, $0)

When the user has no agent CLI installed, the realize lane falls back to
`crates/loom-core/src/synthesis.rs::synthesize_from_template`, which
emits a deterministic real-x07AST body for the common project kinds
(sort, greet, calc, parse, validate, crawl, gateway, workflow-graph).
The template floor is intentionally simple — its job is to make Try-It
return a real output for the easy kinds so the user gets immediate
feedback without spending a dime. Complex targets surface the realize
CTA so the user opts into a subscription run.

### Auto-realize on build

`run_build_pipeline` now detects `scaffold_only` at the end of the
canonical build chain and automatically fires the template synthesizer,
re-running `impl.check` + `xtal.verify` before emitting the summary. The
user sees a single Verified turn with a non-stub headline instead of a
manual "Implement with Claude Code" step. The CTA stays available for
target kinds the template doesn't cover.

### Build-time enforcement

`crates/loom-core/tests/no_metered_api.rs` grep-scans every Rust + TOML
file under `crates/loom-{types,store,adapters,core,daemon}` for the
forbidden strings (`api.anthropic.com`, `api.openai.com`,
`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`). Comment lines are exempt so the
contract can be documented in module docs. CI fails the build if any
new code path references those tokens.

## Validation Targets

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && npm run check && npm test && npm run build && npm run e2e && npm run e2e:connected
```
