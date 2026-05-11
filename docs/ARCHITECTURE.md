# Architecture

## Crates

- `loom-types`: all shared serializable types and API contracts
- `loom-core`: workspace model, reducer, and stateful kernel orchestration
- `loom-store`: `.x07/studio/` filesystem persistence
- `loom-adapters`: CLI execution, MCP transports, and provider probing
- `loom-client`: thin HTTP client for the daemon API
- `loom-daemon`: Axum REST server over the kernel
- `x07-studio`: egui desktop shell
- `x07-studio-forge`: ratatui terminal shell
- `web`: SvelteKit browser shell over the daemon API

## Runtime shape

```text
GUI / TUI
Web
   ↓
loom-client
   ↓
loom-daemon
   ↓
loom-core
   ├─ loom-store
   └─ loom-adapters
       ├─ x07 CLI runner
       ├─ MCP HTTP transport
       ├─ MCP stdio transport
       └─ provider probe engine
```

## State ownership

- Session snapshots are persisted under `.x07/studio/sessions/`
- Provider profiles are persisted under `.x07/studio/providers/`
- Provider probe reports are persisted under `.x07/studio/providers/<id>.probe.json`
- CLI report files are written under `.x07/studio/reports/`

## Lifecycle reducer

Loom owns a finite lifecycle reducer instead of letting shells run arbitrary steps:

```text
intent -> spec -> realization -> verify -> repair -> trust/certify -> ops
```

The reducer routes spec-changing repairs back to spec review, records CLI/MCP effects as operation records, and exposes phase-specific allowed verbs in every session snapshot. GUI and TUI clients render those snapshots; they do not duplicate command execution, provider probing, or MCP transport logic.

## Browser projection

The SvelteKit `web/` client is a lifecycle console, not a second kernel. It calls
the daemon under `/v1/**` for sessions, bindings, and operation execution. When
the daemon is offline it renders a deterministic demo projection using the same
phase names, event names, and artifact concepts so browser tests can still cover
the user-facing XTAL flow.

The browser surface accepts written plans, voice transcripts, and incident notes
as intent sources. It makes the human approval loop explicit:

```text
initial plan -> polished intent packet -> approve/change -> spec draft
  -> approved spec -> realization proposal -> verify -> repair/trust
```

OpenAI Codex and Claude Code are shown as coding-agent lanes with guarded verbs,
write scopes, and review gates. The current backend provider profile is
model-transport oriented, so command-capable agent profiles are tracked as a
follow-up in `docs/XTAL_WORKFLOW_FINDINGS.md`.

The web intake starts with simple, intermediate, and complex x07 project briefs.
They are intentionally editable form seeds, not hidden generators: a user or
agent chooses difficulty, task type, title, input mode, and prompt text before a
session is created.
