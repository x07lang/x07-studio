# XTAL workflow implementation findings

This document records friction found while implementing the Studio web surface.

## Findings

1. Studio needed a browser projection.

   The existing Rust GUI and TUI were useful thin clients, but the current
   product goal requires a browser-run XTAL surface for humans and agents. The
   new `web/` client keeps the daemon as source of truth and avoids creating a
   second lifecycle kernel.

2. The daemon has lifecycle events, but no text-to-intent endpoint.

   The web client can submit a complete `x07.studio.intent_packet@0.1.0`, but
   the daemon does not yet expose a first-class `intent.formalize` operation
   that accepts raw human text plus agent/provider choice. That makes polished
   intent generation a client concern for now. The next backend improvement is
   a daemon endpoint that records the raw plan, chosen agent, generated packet,
   revision history, and approval status as one auditable artifact.

3. Agent providers and coding-agent runners are different concepts.

   `ProviderProfile` is currently OpenAI-compatible model transport. That is
   enough for local/OAI-compatible inference, but OpenAI Codex and Claude Code
   need to be modeled as coding-agent runners with command/MCP capabilities,
   write scopes, and visible worklogs. The web UI shows both lanes, but the
   backend should add a `x07.studio.agent_profile@0.1.0` schema.

4. XTAL lifecycle commands are binding-first, but long-running visibility is
   still coarse.

   `OpRecord` persists completed command details. For a fully visible agent
   workflow, Studio needs streaming operation events: command started, stdout
   chunk, artifact detected, diagnostic classified, approval requested, and
   write completed.

5. Documentation is strong on agent quickstart but scattered for Studio.

   `x07/docs/getting-started/agent-quickstart.md`,
   `available-skills.md`, and the guides explain canonical loops, skills, and
   `x07 run`. Studio should compile those into session doctrine automatically
   and display the selected references in the session contract.

6. The API docs had a stale event envelope example.

   `DispatchEventRequest` wraps the tagged `SessionEvent` as
   `{ "event": { "event": "...", "payload": ... } }`. The older docs showed a
   flattened event/payload shape, which would slow down any browser or agent
   client integration. The example is now aligned with the Rust serde shape.

7. `spec.scaffold` can generate a reserved parameter name.

   A direct scaffold using `--param input:bytes` passes spec checks, but
   `xtal impl sync --write` then emits an implementation module that fails
   x07AST parsing because `input` is reserved. Studio now derives
   `payload:bytes` for intent-created operations. The x07 CLI should either
   reject reserved parameter names during scaffold or normalize them before
   implementation sync.

8. End-to-end XTAL creation needs a daemon orchestration endpoint.

   Running bindings one at a time from the browser made project initialization,
   spec scaffolding, generated tests, implementation sync, and verification too
   easy to desynchronize. The new `/v1/sessions/{session_id}/xtal/run` route
   keeps the lifecycle in the kernel and records each canonical command as
   visible evidence.

9. Agent profiles need to be command runners, not provider profiles.

   OpenAI-compatible provider profiles are about model HTTP transport. Codex
   and Claude Code are coding-agent command lanes with write roots, MCP tools,
   approval gates, and allowed verbs. Studio now exposes them through
   `x07.studio.agent_profile@0.1.0`, but still needs a future execution bridge
   that can launch those agents under the session contract.

10. Coding agents need portable handoff artifacts.

   A profile alone does not keep an external agent inside the approved XTAL
   contract. Studio now writes `.x07/studio/handoffs/*.md` prompts with the
   approved intent, allowed verbs, MCP tools, write roots, and required loop.
   Studio can now record supervised launch plans and run configured agent
   commands with a bounded timeout into visible `OpRecord`s. The daemon now
   appends a `running` record before execution and updates the same record on
   completion, so the web UI can poll active work. Human checkpoints are also
   explicit pending `agent.approval.*` records for approval-gated profiles. The
   remaining gap is streaming stdout/stderr chunks while the command is still
   running and binding those chunks to finer-grained approval prompts.

11. The starter-template path needs a cleaner scaffold contract.

   A live Studio run against `x07 init --template xtal-pure` can initialize and
   verify a project end to end, but the generic `spec.scaffold` step may rewrite
   the starter's existing `toy.sorter` operation with Studio-derived names. The
   project still verifies, but `xtal impl check` reports warnings such as
   `WXTAL_IMPL_PARAM_NAME_MISMATCH` and extra contract clauses. Studio now skips
   scaffold when the template-provided spec path already exists. The x07 CLI
   could still expose a machine-readable "operation already exists" or
   "merge/update scaffold" mode for agentic workflows.

12. Docs-example workflows need environment-aware sandbox and arch gates.

   Live Studio runs against `docs/examples/apps/x07-api-gateway`,
   `docs/examples/apps/x07dbguard`, and
   `docs/examples/readiness-checks/x07-sm-arch-contracts-smoke` exposed two
   project-ladder issues. First, VM-backed sandbox runs fail on local machines
   without `X07_VM_VZ_GUEST_BUNDLE`; Studio now keeps the VM bindings but uses
   explicit `*.sandbox.os` bindings with `--i-accept-weaker-isolation` when the
   VM bundle is not declared. Second, several docs examples still had
   `x07.arch.manifest@0.1.0` manifests while the current toolchain requires
   `0.3.0`; the source examples were updated and Studio now runs
   `arch.check.write_lock` as part of the seeded complex workflows.
