# Cycle 7 notes

The real-toolchain stress pass started here. Per-phase status as of 2026-05-13:

| Phase | Status | Notes |
|---|---|---|
| Plan | done | `dev-docs/phases/x07-studio/cycle-7-plan.md` |
| 7A — harness | done | `scripts/stress_pass.py` (record-only, not orchestrator) |
| 7B — scenario 1 (text-utils baseline) | partial | Real toolchain delivered real artifacts on disk; UI froze mid-flight (see F3) |
| 7C — scenario 2 (CSV repair) | deferred | Pending F3 fix |
| 7D — scenario 3 (os-time widen) | deferred | Pending F3 fix |
| 7E — scenario 4 (PBT regression) | deferred | Pending F3 fix |
| 7F — scenario 5 (Architect+Coder) | deferred | Pending F3 fix |
| 7G — findings | done | `docs/STRESS_PASS_FINDINGS.md` |
| 7H — production-readiness | done | `docs/PRODUCTION_READINESS.md` |

## What we learned from scenario 1

1. **The real x07 toolchain is more capable than the fake.** Real AGENT.md is 100+ lines of proper agent operating guide. Real spec/src trees include multiple modules. Real verify diag carries proof-support warnings the fake never emitted.
2. **The daemon's HTTP layer is the production blocker, not the toolchain integration.** When real `claude` / `codex` subprocesses are running, the daemon's `tokio` HTTP server stops responding (returns `Empty reply from server`). The subprocess work continues to completion on disk, but the UI sees a frozen page. This is F3 — top-priority bug.
3. **The HealthRow correctly surfaces real toolchain state.** Lockfile-stale and migrate-needed pills appear because real `x07 pkg lock --check` and `x07 migrate --check` actually run and report. The connected-E2E fake masked this completely.

## Workaround for F3 (use during internal dogfood)

When the UI freezes mid-build:
1. Identify the daemon PID: `ps -ef | grep loom-daemon`
2. Kill it: `kill <pid>`
3. Restart it pointing at the same workspace: `target/debug/loom-daemon serve --root <workspace> --addr 127.0.0.1:<port>`
4. Refresh the browser. The session list will reload from disk; you can pick up where you left off.

This is good enough for internal alpha. Not OK for any external user.

## Suggested next steps

1. **F3 root-cause investigation.** Likely a `tokio::sync::Mutex` held across an `await`, or an SSE stream not dropping on client disconnect. Tools: tokio-console + targeted log instrumentation in `kernel.rs::run_role_pipeline` and `execute_agent_command_streaming`.
2. After F3 fix lands, **re-run scenarios 2-5** in sequence (each 5-10 min) and update findings.
3. **F1 + F4** are small UX fixes that can land independently of F3.
