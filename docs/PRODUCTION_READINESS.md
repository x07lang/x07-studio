# Production-readiness checklist

Last updated 2026-05-13 after the pre-production readiness pass.

## Decision summary

| Track | Verdict | Why |
|---|---|---|
| Internal alpha (Studio dogfooded by x07 team) | **YES** | Canonical loop works against real x07 toolchain on disk. F3 fixed at both autopilot and per-substep level (commits `1ae54f8`, `2344fde`). HTTP stays responsive (8/8 probes <2ms) while real claude/codex/x07 are running. |
| Public beta (invite-only external users) | **YES** | F1/F2/F3/F4/F5/F13-F18 are closed or documented. Scenarios 2-5 now have real-toolchain evidence, cross-browser connected smoke passes in Chromium, Firefox, and WebKit, and the 30-minute single-session stability soak passes. |
| Production / general availability | **NO** | Public-beta gates are closed, but GA-only gates remain: real longitudinal usage, real signed certify, cross-platform, accessibility, performance, observability, and user-facing docs. |

## What's verified against real toolchain

- ✅ Real x07 0.2.10 toolchain integration on disk: AGENT.md is properly generated, x07.json + x07-toolchain.toml + x07.lock.json land, full spec/src/target trees materialize, real xtal.verify produces a proper diag report with `ok:true` and real proof-support warnings.
- ✅ Subscription-only cost contract holds (gate stays green; only flat-rate CLIs invoked).
- ✅ HealthRow correctly reflects real lockfile/migrate state; fresh workspaces render `MIGRATE init → 0.5`.
- ✅ Build/clippy/type-check gates green.
- ✅ All connected-E2E tests still green (the fake harness suite).
- ✅ Daemon health reports active session and SSE subscriber counts for stability-soak monitoring.
- ✅ TrustCard surfaces real proof-support diagnostics and distinguishes active posture computation from idle pending state.
- ✅ Real-toolchain scenarios 2-5 have evidence bundles: CSV repair pause, run-os/os-time trust widening, PBT counterexample regression capture, and Architect/Coder/Reviewer role pipeline acceptance.

## What's NOT verified — must be done before each tier

### Before internal alpha
- [x] At least one real-toolchain scenario produces real on-disk artifacts. (Scenario 1 done.)
- [x] Subscription-only contract enforced. (`no_metered_api` gate green.)
- [x] Document the F3 workaround historically in `docs/CYCLE_7_NOTES.md`; it is now obsolete because F3 is fixed.

### Before public beta
- [x] **F3 diagnosed and fixed.** Daemon HTTP remains responsive during real-claude / real-codex subprocess execution (commits `1ae54f8`, `2344fde`).
- [x] **Scenarios 2-5 run with real toolchain.** Scenario 2 uses the seeded CSV fixture, reaches real `xtal.verify` failure, runs real `xtal.repair`, and pauses at `realize_stalled`. Scenario 3 captures `run-os` + `os-time` trust widening and amber posture. Scenario 4 captures a real PBT counterexample and locks it as a regression test. Scenario 5 invokes real Claude/Codex/Claude role stages, records a bounded Codex timeout plus deterministic Unicode template fallback, then accepts through real Claude review.
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

1. **Recording bundles in git.** The Cycle 7 plan said `.gitignore` them. Worth committing the *sanitized summary* artifacts (real-AGENT.md, real-verify-diag.json snippets) as evidence?
2. **Public-beta launch timing.** The invite-only public-beta gates are now closed with documented limitations. Pick a rollout window and dogfood cohort before GA work starts.

## Source documents

- `docs/STRESS_PASS_FINDINGS.md` — full finding log F1..F18.
- `dev-docs/phases/x07-studio/cycle-7-plan.md` — the original Cycle 7 plan.
- `target/stress-pass/scenario-1-text-utils/` — scenario 1 evidence (artifacts, screenshots, scenario config).
- `target/stress-pass-f13-check/scenario-2-csv-repair/` — scenario 2 repair-loop evidence (real `xtal.verify` failure, real `xtal.repair`, clean `realize_stalled` pause).
- `target/stress-pass-s3-accepted/scenario-3-os-time/` — scenario 3 trust-widen evidence (`run-os`, `os-time`, amber posture).
- `target/stress-pass-s4-accepted/scenario-4-pbt/` — scenario 4 PBT counterexample and regression-lock evidence.
- `target/stress-pass-s5-summary-fix/scenario-5-collab/` — scenario 5 role-pipeline evidence (Claude clarify, Codex timeout, template fallback, Claude review accept).
- `scripts/stress_pass.py` — the recording harness.
- `scripts/scenarios/scenario-1..5-*.json` — scenario configs.
