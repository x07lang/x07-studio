# Step Budgets

Pipeline stages carry `x07.studio.step_budget@0.1.0`.

Default budgets:

- Architect confirm spec: 8 seconds.
- Coder implementation: 60 seconds.
- Reviewer pass: 30 seconds.

On exhaust, the pipeline records `pipeline.budget_exhausted` with the
responsible step and preserves partial work. Autopilot then pauses with stage
`budget_exhausted` so the user can extend, skip, or continue manually.

The Process Lane shows running-step budget chips. The Now panel surfaces the
latest pause reason from the autopilot decision.
