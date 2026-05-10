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

## Runtime shape

```text
GUI / TUI
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
