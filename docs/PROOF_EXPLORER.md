# Proof Explorer

Proof Explorer opens from behavior promises in a verified plain-English summary.
Each promise is assigned a deterministic id by the summarizer and resolved with:

```http
GET /v1/sessions/{session_id}/proof/{behavior_id}
```

The daemon joins:

- `summary.plain_english` behavior promises
- latest `xtal.verify` report JSON
- `target/xtal/verify/summary.json`
- session assumptions

Statuses are intentionally explicit:

- `proved`: proof objects or proof counts are present
- `test-evidence`: verify passed, but no proof object was found
- `assumed`: no current verify evidence was found

The browser should preserve this distinction instead of upgrading
test-evidence into a proof claim.
