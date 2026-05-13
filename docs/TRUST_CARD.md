# Trust Card

The Trust Card is the compact posture summary shown in the Now panel and as
timeline posture-change turns.

Data source:

- `GET /v1/sessions/{session_id}/trust/posture`
- `posture.captured` operation records
- `target/trust/report.json`, `target/cert/trust-report.json`,
  `target/xtal/verify/summary.json`, `x07.json`, and ladder state

The card shows:

- active worlds, defaulting to `solve-pure`
- declared capabilities such as `os-net`, `os-fs`, and `os-time`
- local and prover budgets
- support/proof coverage and open assumptions
- posture color: green for pure/proved, amber for wider but bounded, red for
  risky network or proof coverage drops

Clients should not infer trust posture from CSS or route state. Use the API
payload and display `posture_color` directly.
