# Performance Budgets

## Budgets

| Surface | Budget |
|---|---:|
| Landing page time to interactive | <= 5000 ms |
| Process Lane step render p95 | <= 500 ms |
| Daemon `/v1/health` p95 under connected load | <= 200 ms |

The Process Lane render budget is measured from browser `performance.measure`
entries emitted by `StepNode.svelte` when the page is loaded with `?perf=1`.

## Gate

Run:

```bash
python3 scripts/perf_budget.py
```

The script runs the connected Playwright budget test and writes:

```text
target/perf/<timestamp>/budget-report.json
```

The report schema is `x07.studio.perf_budget@0.1.0` and includes the captured
budgets, measured values, and final `passed` boolean. CI runs this gate after
the connected E2E suite so the same deterministic daemon and browser path are
used for regression checks.

Latest local report:

- Path: `target/perf/manual-budget-report.json`
- Landing TTI: 896 ms
- Process Lane render p95: 0.10 ms
- Daemon `/v1/health` p95: 2 ms

## Regression Policy

- Any value above its budget is a blocking regression.
- A value still below budget but more than 20 percent slower than the latest
  accepted baseline should be investigated before release.
- Keep reports under `target/perf/`; do not commit raw Playwright traces unless
  they are needed for a specific incident.
