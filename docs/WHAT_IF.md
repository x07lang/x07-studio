# What-If Forecasts

What-if forecasts preview pending Process Lane steps before they run.

Endpoint:

```text
POST /v1/sessions/{id}/process-lane/whatif
```

Request:

```json
{"step_id":"verify"}
```

Response schema: `x07.studio.what_if_forecast@0.1.0`.

Forecasts include:

- estimated duration in milliseconds,
- confidence from `0.0` to `1.0`,
- assumptions shown directly in the tooltip,
- optional semantic or trust delta when a fast projection is available.

Forecasts are advisory. The actual operation record remains the source of truth
once the step runs.
