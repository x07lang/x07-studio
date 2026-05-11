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
as intent sources. In connected mode, `Polish Intent` calls the daemon's
`intent.formalize` endpoint, so the kernel owns the generated intent packet,
records revision notes, and appends a visible `intent.formalize` operation
before the human approval gate. It makes the approval loop explicit:

```text
initial plan -> polished intent packet -> approve/change -> spec draft
  -> approved spec -> realization proposal -> verify -> repair/trust
```

OpenAI Codex and Claude Code are shown as coding-agent lanes with guarded verbs,
write scopes, and review gates. The current backend provider profile is
model-transport oriented, so command-capable agent profiles are tracked as a
follow-up in `docs/XTAL_WORKFLOW_FINDINGS.md`.

Studio also exposes command-capable coding agents through
`x07.studio.agent_profile@0.1.0`. The daemon returns default Codex and Claude
Code profiles, marks whether their commands are available on `PATH`, and stores
workspace overrides under `.x07/studio/agents/`.
Per-session handoffs are generated under `.x07/studio/handoffs/` so Codex or
Claude Code receives the approved intent, session contract, allowed verbs, MCP
tools, write roots, and required XTAL loop as a concrete prompt artifact.
Studio can also record a supervised launch plan, or execute the configured agent
command with a bounded timeout, and append the resulting stdout/stderr and
artifacts as `agent.supervise.*` or `agent.run.*` operation records. Agent run
execution is split into a `running` record and a later completion update so web
clients can poll active progress without holding the daemon session lock. While
the process is active, stdout/stderr chunks update the same `agent.run.*` record
so the browser worklog can show live command evidence instead of waiting for
process exit.
Profiles marked `approval_required` are gated by pending `agent.approval.*`
records; humans approve or reject those checkpoints in the same visible worklog
before Studio starts the supervised command. Approval checkpoints are one-shot:
the next relevant handoff, plan, or run requires a fresh approval.

The web intake starts with simple, intermediate, advanced, complex, and expert
x07 project briefs. They are intentionally editable form seeds, not hidden
generators: a user or agent chooses difficulty, task type, title, input mode,
and prompt text before a session is created.

After spec approval, the daemon can run the visible XTAL workflow through
`POST /v1/sessions/{session_id}/xtal/run`. That path derives binding variables
from the intent packet, initializes an `xtal-pure` project only when `x07.json`
is absent, or seeds a supported `x07/docs/examples` project when the intent maps
to one. It then runs the template's generation, arch/package, test, run, bundle,
and verification commands while appending each command as an `OpRecord`.
