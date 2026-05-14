# Agent Roles

Studio routes by role instead of hard-coded agent id.

Roles:

- `conductor`: Studio itself and canonical CLI work.
- `architect`: spec, AGENT.md, assumptions, and judgment-heavy review.
- `coder`: implementation, repair, and mechanical edits.
- `reviewer`: final review before the pipeline is accepted.

Built-in defaults:

- `claude-code`: architect, reviewer, eligible coder.
- `openai-codex`: coder, reviewer.

The role pipeline records `x07.studio.role_pipeline@0.1.0` in autopilot
decision evidence. The happy path is:

```text
Architect confirms spec -> Coder writes impl -> Reviewer runs supervised review -> review_round accepts
```

When a reviewer agent is configured, Studio runs that CLI in read-only review
mode and ingests its `agent_event` verdict before recording `review.round`.
If no reviewer agent is configured or the reviewer handoff cannot be prepared,
Studio falls back to the deterministic baseline review by checking generated
tests, latest verify, and scaffold-only summaries. If only the writer is
configured, self-review is allowed when memory preference `allow_self_review`
is true.

Per-session overrides live as `role.overrides` operations. User defaults live in
`~/.x07-studio/memory.jsonl` as `x07.studio.role_preferences@0.1.0`.
