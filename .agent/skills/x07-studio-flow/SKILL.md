# x07-studio-flow

Use this skill when editing the Studio workspace.

## Primary loop

- preserve the Loom reducer semantics
- preserve daemon API compatibility
- prefer canonical x07 machine outputs over ad-hoc parsing
- keep GUI and TUI as thin clients over the daemon whenever possible

## Implementation rules

- Add new daemon endpoints before adding new shell affordances.
- Shared wire types live in `loom-types`.
- Stateful MCP transports belong in `loom-adapters`.
- Session persistence belongs in `loom-store`.
- Shell code must not duplicate command execution or provider probing logic.
