# Production-readiness checklist

Last updated 2026-05-13 after the pre-production readiness pass.

## Decision summary

| Track | Verdict | Why |
|---|---|---|
| Internal alpha (Studio dogfooded by x07 team) | **YES** | Canonical loop works against real x07 toolchain on disk. F3 fixed at both autopilot and per-substep level (commits `1ae54f8`, `2344fde`). HTTP stays responsive (8/8 probes <2ms) while real claude/codex/x07 are running. |
| Public beta (invite-only external users) | **PENDING SCENARIOS 2-5** | F3 and F4 are fixed; F1/F2/F5 are closed. Cross-browser connected smoke passes in Chromium, Firefox, and WebKit. The 30-minute single-session stability soak passes. The remaining public-beta gate is scenarios 2-5 on the real toolchain. |
| Production / general availability | **NO** | Public-beta scenario evidence is still incomplete, and GA-only gates remain: real longitudinal usage, real signed certify, cross-platform, accessibility, performance, observability, and user-facing docs. |

## What's verified against real toolchain

- ✅ Real x07 0.2.10 toolchain integration on disk: AGENT.md is properly generated, x07.json + x07-toolchain.toml + x07.lock.json land, full spec/src/target trees materialize, real xtal.verify produces a proper diag report with `ok:true` and real proof-support warnings.
- ✅ Subscription-only cost contract holds (gate stays green; only flat-rate CLIs invoked).
- ✅ HealthRow correctly reflects real lockfile/migrate state; fresh workspaces render `MIGRATE init → 0.5`.
- ✅ Build/clippy/type-check gates green.
- ✅ All connected-E2E tests still green (the fake harness suite).
- ✅ Daemon health reports active session and SSE subscriber counts for stability-soak monitoring.
- ✅ TrustCard surfaces real proof-support diagnostics and distinguishes active posture computation from idle pending state.

## What's NOT verified — must be done before each tier

### Before internal alpha
- [x] At least one real-toolchain scenario produces real on-disk artifacts. (Scenario 1 done.)
- [x] Subscription-only contract enforced. (`no_metered_api` gate green.)
- [x] Document the F3 workaround historically in `docs/CYCLE_7_NOTES.md`; it is now obsolete because F3 is fixed.

### Before public beta
- [x] **F3 diagnosed and fixed.** Daemon HTTP remains responsive during real-claude / real-codex subprocess execution (commits `1ae54f8`, `2344fde`).
- [ ] **Scenarios 2-5 run with real toolchain.** Currently 1/5 scenarios accepted. Scenario 2 was probed against the real daemon and is blocked by F13: no parser contract reaches `xtal.verify`, so the repair loop does not fire.
- [x] Cross-browser smoke (Chromium + Firefox + WebKit). Harness: `python3 scripts/cross_browser_smoke.py`; latest local result passed.
- [x] 30-minute long-running stability test (single autopilot session, observe op-log size + memory). Harness: `python3 scripts/stability_soak.py`; latest local result passed.
- [x] F4: proof-support warnings surfaced in TrustCard.
- [x] F2: TrustCard shows active "Computing trust posture..." state while build/verify is running before first posture.
- [x] F1: HealthRow migrate label fixed for null `from_schema`.
- [x] F5: x07 toolchain discovery order documented in `docs/V0_1_STATUS.md`.
- [x] Basic security review: AGENT.md write-back path, file uploads, MCP tool transparency. See `docs/SECURITY_REVIEW.md`.

## Browser support matrix

| Browser engine | Current status | Evidence |
|---|---|---|
| Chromium | Pass | `target/stress-pass/cross-browser-smoke/summary.json` from `python3 scripts/cross_browser_smoke.py` |
| Firefox | Pass | `target/stress-pass/cross-browser-smoke/summary.json` from `python3 scripts/cross_browser_smoke.py` |
| WebKit / Safari engine | Pass; Web Speech remains headless-limited | `target/stress-pass/cross-browser-smoke/summary.json` from `python3 scripts/cross_browser_smoke.py` |

## Stability soak results

Latest harness: `python3 scripts/stability_soak.py`.

A 30-minute single-session autopilot soak passed on 2026-05-13:

- Command: `python3 scripts/stability_soak.py --duration-seconds 1800 --poll-seconds 5`
- Evidence: `target/stress-pass/stability-soak/20260513-152704/summary.json`
- Result: 1 session, 351 autopilot invocations, 0 failures, peak RSS 15,456 KiB, max op-log size 369, subscriber count stable at 0.

The daemon now exports `active_sessions` and `subscriber_count` on `/v1/health` and `/v1/health/snapshot`; the soak writes `metrics.csv` and `summary.json` under `target/stress-pass/stability-soak/`. No op-log truncation strategy is needed at the current baseline because the 30-minute max op-log size stayed below 10k.

### Before production / general availability
- [ ] All public-beta items above.
- [ ] At least 100 real user-driven sessions across ≥3 project archetypes (sort, parser, service) without F3-class freezes.
- [ ] Real `x07lp` integration (currently faked).
- [ ] Real `x07 trust certify` produces a real signed certificate; certificate viewer renders the real artifact.
- [ ] Cross-platform: Linux + Windows-via-WSL confirmation.
- [ ] Accessibility audit: baseline focus/reduced-motion/ARIA pass documented in `docs/ACCESSIBILITY.md`; axe-core serious/critical audit still pending.
- [ ] Performance: every Process Lane step under 500ms client-render; daemon p95 ≤ 200ms even under autopilot load.
- [x] Documentation: user guide, agent guide, troubleshooting runbook, and docs index.
- [ ] Observability: per-session structured logs, error reporting opt-in, basic SLO dashboard.

## Open questions for Bodik

1. **Workspace shape for scenarios 2-5.** Scenario 2 (CSV repair) assumes the workspace already has a failing impl to repair. Should scenarios run sequentially against a single workspace (cumulative state) or each get a fresh workspace?
2. **Scenario 2 contract source.** Should the CSV repair scenario get a deterministic parser predicate (C2) or a pre-seeded failing contract fixture so repair is deliberately exercised?
3. **Recording bundles in git.** The Cycle 7 plan said `.gitignore` them. Worth committing the *sanitized summary* artifacts (real-AGENT.md, real-verify-diag.json snippets) as evidence?
4. **Internal-alpha launch timing.** The real-toolchain integration works on disk and the historical F3 workaround is obsolete. Worth a soft launch to the x07 team this week while scenarios 2-5 and the long soak are queued?

## Source documents

- `docs/STRESS_PASS_FINDINGS.md` — full finding log F1..F5.
- `dev-docs/phases/x07-studio/cycle-7-plan.md` — the original Cycle 7 plan.
- `target/stress-pass/scenario-1-text-utils/` — scenario 1 evidence (artifacts, screenshots, scenario config).
- `scripts/stress_pass.py` — the recording harness.
- `scripts/scenarios/scenario-1..5-*.json` — scenario configs.
