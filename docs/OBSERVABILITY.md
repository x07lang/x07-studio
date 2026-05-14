# Observability

## Logs

`loom-daemon` initializes `tracing-subscriber` from `RUST_LOG`.

Default:

```bash
RUST_LOG=loom_daemon=info,axum=info
```

Useful dogfood setting:

```bash
RUST_LOG=info,loom_core=debug,loom_adapters=debug
```

HTTP requests are wrapped by `tower-http` `TraceLayer`. Kernel-level operations
are still represented primarily by session operation records; those records are
the source of truth for timeline, process lane, proof, release, and agent
handoff events.

## Metrics

`GET /v1/metrics` returns Prometheus text with:

- `loom_daemon_active_sessions`
- `loom_daemon_sse_subscribers`
- `loom_telemetry_session_summaries_total`
- `loom_telemetry_error_ring_entries`

The metrics endpoint is intentionally small and local. It covers the public beta
stability counters plus the opt-in telemetry evidence needed for GA readiness.

## Error Ring

Opt-in browser errors are retained in:

```text
.loom/error-ring.jsonl
```

The daemon keeps the latest 100 entries. Each entry is sanitized by the browser
and bounded again by the daemon before it reaches disk.

## SLO Dashboard

Run:

```bash
python3 scripts/slo_dashboard.py --root /path/to/workspace --addr http://127.0.0.1:7719
```

The dashboard prints active sessions, SSE subscribers, retained session
summaries, retained browser errors, and the five most recent local error-ring
entries.
