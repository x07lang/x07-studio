# Autopilot

Autopilot is a bounded Loom driver over the existing XTAL lifecycle. It does
not bypass the reducer, canonical bindings, write-root audits, or verify gates.

## Policy

`x07.studio.autopilot_state@0.1.0` stores:

- `auto_answer_min_confidence`: default `0.7`.
- `max_clarify_rounds`: default `3`.
- `auto_climb_to`: optional ladder rung ceiling.
- `allow_repair_iters`: default `3`.
- `allow_quorum`: compare Claude Code and Codex when realization is required.

## Decisions

The planner reads the current `SessionSnapshot` and emits one decision at a
time:

- `clarify_answer`: answer option-bound clarify questions with their first
  option when confidence is high enough.
- `spec_approve`: draft and approve the spec when intent is stable.
- `build_run`: run the canonical build pipeline.
- `realize`: run template or supervised agent realization when verify still
  points at scaffold-only code.
- `ladder_climb`: climb to the configured rung.
- `complete` or `pause`: stop for user review.

Every decision is recorded as `autopilot.decision.<stage>` so the Timeline and
audit log show why the system moved without a click.
