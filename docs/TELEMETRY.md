# Telemetry

Studio telemetry is local and opt-in. The web UI only submits session summaries
or browser error reports when the user checks the share control in the session
summary card. The daemon also rejects telemetry payloads with `consent: false`.

## Data Written

- Session summaries: `.loom/session-summary.jsonl`
- Browser error ring: `.loom/error-ring.jsonl`

Session summaries contain:

- `session_id`
- archetype
- time to first verified or certified op
- repair round count
- estimated agent minutes
- success flag
- short friction notes from the closing form

Browser error reports contain:

- severity and source
- sanitized message and stack
- current route
- browser user agent
- sanitized context for failed fetches or unhandled errors

The browser redacts absolute paths, email addresses, and UUID-shaped values
before sending reports. The daemon trims large strings, caps arrays and objects,
and keeps only the last 100 browser error reports.

## Retention

Telemetry stays in the selected workspace. Studio does not upload it to any
remote service. Operators can delete the local files directly:

```bash
rm .loom/session-summary.jsonl .loom/error-ring.jsonl
```

## API

- `POST /v1/telemetry/session-summary`
- `POST /v1/telemetry/error`

Both endpoints return `x07.studio.telemetry_write@0.1.0` with:

- `accepted`
- `path`
- `retained`

`accepted: false` means the payload was not written because consent was not
enabled.
