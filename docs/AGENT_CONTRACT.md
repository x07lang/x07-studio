# AGENT.md contract

Studio treats `AGENT.md` as the project architecture contract for supervised
agents.

## Daemon behavior

- `GET /v1/sessions/{id}/agent-contract` reads `AGENT.md` from the workspace.
- When the file is missing, Studio returns a deterministic markdown template
  seeded from the current intent targets, constraints, policy implications, and
  forbidden witnesses.
- `POST /v1/sessions/{id}/agent-contract` writes `AGENT.md` atomically and
  rejects stale saves when `prior_hash` no longer matches the on-disk file.
- Clarify and realize handoff prompts include `## Project AGENT.md` before the
  per-session contract.

## Authoring shape

The drawer parses markdown `##` sections into anchors. Recommended sections are
Purpose, Non-goals, Invariants, Module map, Tooling commands, Budgets / gates,
and Forbidden changes.
