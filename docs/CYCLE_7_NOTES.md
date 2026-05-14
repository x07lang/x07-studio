# Cycle 7 notes

The real-toolchain stress pass started here. Per-phase status as of 2026-05-13:

| Phase | Status | Notes |
|---|---|---|
| Plan | done | `dev-docs/phases/x07-studio/cycle-7-plan.md` |
| 7A — harness | done | `scripts/stress_pass.py` (record-only, not orchestrator) |
| 7B — scenario 1 (text-utils baseline) | done — F3 fixed, scenario 1 baseline captured | Real toolchain delivered real artifacts on disk; daemon remains responsive during subprocess load after commits `1ae54f8` and `2344fde` |
| 7C — scenario 2 (CSV repair) | done — repair loop exercised, clean pause | Seeded CSV example fixture makes real `xtal.verify` fail; real `xtal.repair` runs and autopilot pauses at `realize_stalled` |
| 7D — scenario 3 (os-time widen) | done | Real toolchain captured `run-os` + `os-time` trust widening, amber posture, proof-support warnings, and local-preview gate loss |
| 7E — scenario 4 (PBT regression) | done | Real PBT manifest runs, captures a failing counterexample, and converts it into a deterministic regression via `x07 fix --from-pbt` |
| 7F — scenario 5 (Architect+Coder) | done | Real Claude clarify, real Codex invocation, deterministic Unicode template fallback after bounded Codex timeout, and real Claude review accept |
| 7G — findings | done | `docs/STRESS_PASS_FINDINGS.md` |
| 7H — production-readiness | done | `docs/PRODUCTION_READINESS.md` |

## What we learned from scenario 1

1. **The real x07 toolchain is more capable than the fake.** Real AGENT.md is 100+ lines of proper agent operating guide. Real spec/src trees include multiple modules. Real verify diag carries proof-support warnings the fake never emitted.
2. **The daemon's HTTP layer is the production blocker, not the toolchain integration.** When real `claude` / `codex` subprocesses are running, the daemon's `tokio` HTTP server stops responding (returns `Empty reply from server`). The subprocess work continues to completion on disk, but the UI sees a frozen page. This is F3 — top-priority bug.
3. **The HealthRow correctly surfaces real toolchain state.** Lockfile-stale and migrate-needed pills appear because real `x07 pkg lock --check` and `x07 migrate --check` actually run and report. The connected-E2E fake masked this completely.

## Workaround for F3 (historical)

**Obsolete — F3 fixed (commit `2344fde`).** Keep this record only for older dogfood builds.

When the UI freezes mid-build:
1. Identify the daemon PID: `ps -ef | grep loom-daemon`
2. Kill it: `kill <pid>`
3. Restart it pointing at the same workspace: `target/debug/loom-daemon serve --root <workspace> --addr 127.0.0.1:<port>`
4. Refresh the browser. The session list will reload from disk; you can pick up where you left off.

This was good enough for internal alpha before the fix. It is not an expected path in current builds.

## Suggested next steps

1. Pick the public-beta dogfood cohort and rollout window.
2. Start the GA-only work: longitudinal usage data, real signed certify, cross-platform confirmation, accessibility audit, performance budgets, and observability.
3. Decide whether sanitized stress-pass summary artifacts should be committed or kept as local `target/` evidence only.
