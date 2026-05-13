# Production-readiness checklist

Last updated 2026-05-13 after the Cycle 7 stress-pass first run.

## Decision summary

| Track | Verdict | Why |
|---|---|---|
| Internal alpha (Studio dogfooded by x07 team) | **YES** | Canonical loop works against real x07 toolchain on disk. F3 fixed at both autopilot and per-substep level (commits `1ae54f8`, `2344fde`). HTTP stays responsive (8/8 probes <2ms) while real claude/codex/x07 are running. |
| Public beta (invite-only external users) | **PARTIAL** | F3 fixed. Scenarios 2-5 still unrun against real toolchain (the harness + scenario configs are ready; needs an operator with ~10-20 min per scenario). Cross-browser + 30-min stability still needed. |
| Production / general availability | **NO** | Scenarios 2-5 unrun + cross-browser unvalidated + no long-running stability data + real `x07lp` integration unrun. Multiple unknowns. |

## What's verified against real toolchain

- ✅ Real x07 0.2.10 toolchain integration on disk: AGENT.md is properly generated, x07.json + x07-toolchain.toml + x07.lock.json land, full spec/src/target trees materialize, real xtal.verify produces a proper diag report with `ok:true` and real proof-support warnings.
- ✅ Subscription-only cost contract holds (gate stays green; only flat-rate CLIs invoked).
- ✅ HealthRow correctly reflects real lockfile/migrate state (with cosmetic F1).
- ✅ Build/clippy/type-check gates green.
- ✅ All connected-E2E tests still green (the fake harness suite).

## What's NOT verified — must be done before each tier

### Before internal alpha
- [x] At least one real-toolchain scenario produces real on-disk artifacts. (Scenario 1 done.)
- [x] Subscription-only contract enforced. (`no_metered_api` gate green.)
- [ ] Document the F3 workaround (kill daemon, restart, reload page) in `docs/CYCLE_7_NOTES.md` so dogfooders know what to do when it freezes.

### Before public beta
- [ ] **F3 diagnosed and fixed.** Daemon HTTP must remain responsive during real-claude / real-codex subprocess execution.
- [ ] **Scenarios 2-5 run with real toolchain.** Currently 1/5 scenarios run.
- [ ] Cross-browser smoke (Safari + Firefox).
- [ ] 30-minute long-running stability test (single autopilot session, observe op-log size + memory).
- [ ] F4: proof-support warnings surfaced in TrustCard.
- [ ] F1: HealthRow migrate label fixed for null `from_schema`.
- [ ] Basic security review: AGENT.md write-back path, file uploads, MCP tool transparency.

### Before production / general availability
- [ ] All public-beta items above.
- [ ] At least 100 real user-driven sessions across ≥3 project archetypes (sort, parser, service) without F3-class freezes.
- [ ] Real `x07lp` integration (currently faked).
- [ ] Real `x07 trust certify` produces a real signed certificate; certificate viewer renders the real artifact.
- [ ] Cross-platform: Linux + Windows-via-WSL confirmation.
- [ ] Accessibility audit: ARIA, keyboard nav, screen reader smoke.
- [ ] Performance: every Process Lane step under 500ms client-render; daemon p95 ≤ 200ms even under autopilot load.
- [ ] Documentation: user guide, agent guide, troubleshooting runbook.
- [ ] Observability: per-session structured logs, error reporting opt-in, basic SLO dashboard.

## Open questions for Bodik

1. **F3 priority.** Want me to take a swing at diagnosing the daemon hang next, or save for a focused debug session?
2. **Workspace shape for scenarios 2-5.** Scenario 2 (CSV repair) assumes the workspace already has a failing impl to repair. Should scenarios run sequentially against a single workspace (cumulative state) or each get a fresh workspace?
3. **Recording bundles in git.** The Cycle 7 plan said `.gitignore` them. Worth committing the *sanitized summary* artifacts (real-AGENT.md, real-verify-diag.json snippets) as evidence?
4. **Internal-alpha launch timing.** Even with F3, the toolchain integration works on disk. Worth a soft launch to the x07 team this week, with the F3 workaround documented?

## Source documents

- `docs/STRESS_PASS_FINDINGS.md` — full finding log F1..F5.
- `dev-docs/phases/x07-studio/cycle-7-plan.md` — the original Cycle 7 plan.
- `target/stress-pass/scenario-1-text-utils/` — scenario 1 evidence (artifacts, screenshots, scenario config).
- `scripts/stress_pass.py` — the recording harness.
- `scripts/scenarios/scenario-1..5-*.json` — scenario configs.
