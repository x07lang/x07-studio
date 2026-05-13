# Cycle 6 Notes

Cycle 6 makes the active x07 process visible and switches default collaboration
from parallel quorum to role-based routing.

## Wired Now

- Process Lane projection from `SessionSnapshot.op_log` through
  `loom-core::process_lane`.
- `GET /v1/sessions/{id}/process-lane`, per-step evidence, and what-if
  forecast endpoints.
- Web Process Lane above the Timeline with role colors, current/next copy,
  budget chips, hover forecasts, and click-through evidence drawer.
- Agent roles on profiles: `conductor`, `architect`, `coder`, `reviewer`.
- Default role pipeline: architect confirms spec, coder realizes, reviewer
  records a review round.
- Baseline review path for sessions without a configured reviewer agent.
- Per-session role overrides recorded as `role.overrides` operations.
- Role preferences persisted in `~/.x07-studio/memory.jsonl`.
- Verbal interrupt hook in the Composer for exact phrases `wait stop` and
  `pause autopilot`.
- Parallel quorum remains available as the manual Second opinion action.

## Verification Hooks

- `cargo test -p loom-core --test process_lane_snapshot`
- `cargo test -p loom-core --test no_metered_api`
- `web/src/lib/components/ProcessLane.test.ts`
- `web/src/lib/components/StepDrawer.test.ts`
- `web/src/lib/voice.test.ts`
