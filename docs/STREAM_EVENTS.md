# Agent Stream Events

Studio normalizes local coding-agent JSONL output into
`x07.studio.agent_stream_event@0.1.0`.

## Sources

- Claude Code: `claude -p --output-format stream-json --include-partial-messages`
- Codex: `codex exec --json`

## Normalized Variants

- `reasoning`: intermediate reasoning or planning text.
- `tool_use`: tool call with bounded JSON input.
- `tool_result`: tool result with success flag and a short snippet.
- `agent_message`: assistant/user-visible message text.
- `done`: terminal event with exit code.

Each normalized event is emitted as an `OpRecord` named:

```text
agent.event.<agent_id>.stream_<kind>
```

The existing `/v1/sessions/{id}/stream` SSE endpoint carries these records, so
clients do not need a second event channel.

## Live Diffs

When a `tool_use` event names an Edit or Write-like tool and includes a path
plus before/after or content text, Studio attaches a `live_diff` value and
persists it as:

```text
.x07/studio/diffs/<session_id>/<event_id>.json
```

The web Timeline renders both the compact tool-use card and the live diff panel
while the realize turn is running.
