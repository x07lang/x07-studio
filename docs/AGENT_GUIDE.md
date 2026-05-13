# Agent guide

Studio supervises Codex and Claude Code as lifecycle actors, not free-form shell users.

## What the agent receives

Each handoff includes:

- session id and workspace root;
- approved intent/spec context;
- allowed verbs;
- MCP tools;
- approved write roots;
- relevant `AGENT.md` content;
- expected `x07.studio.agent_event@0.1.0` event protocol.

The same contract is mirrored in `X07_STUDIO_*` environment variables so wrappers can inspect the allowed verbs and roots without parsing prose.

## Write roots

Clarify and architect-enrichment runs normally receive no write roots. Realization runs receive implementation roots such as `src/` and `tests/`. Post-run write-root audits fail the agent run when it edits source/config paths outside the approved set.

Internal Studio files under `.x07/studio/sessions/` are bookkeeping and are excluded from implementation-write summaries.

## MCP calls

MCP tool calls parsed from agent streams are preserved as timeline events with server, tool, arguments, and results. The web card redacts common sensitive argument names before rendering.

## Cost contract

Studio uses subscription-oriented local CLIs. The `scripts/check_no_metered_api.py` gate must stay green; direct metered HTTP API integration remains out of scope for v0.1.

## Expected behavior

Agents should:

- formalize intent before implementation;
- keep spec-changing repairs separate for human approval;
- run canonical x07 tools instead of inventing shell workflows;
- emit structured events for plans, file writes, MCP calls, diagnostics, approvals, and completion.
