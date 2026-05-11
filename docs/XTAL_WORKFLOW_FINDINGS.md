# XTAL workflow implementation findings

This document records friction found while implementing the Studio web surface.

## Findings

1. Studio needed a browser projection.

   The existing Rust GUI and TUI were useful thin clients, but the current
   product goal requires a browser-run XTAL surface for humans and agents. The
   new `web/` client keeps the daemon as source of truth and avoids creating a
   second lifecycle kernel.

2. Intent polishing belongs in the daemon, not only the browser client.

   The web client can still fall back to a deterministic demo projection, but
   connected Studio now calls `/v1/sessions/{session_id}/intent/formalize` with
   raw written text, voice transcripts, or incident notes plus revision history.
   The kernel creates the `x07.studio.intent_packet@0.1.0`, performs the legal
   lifecycle transition, and appends a visible `intent.formalize` `OpRecord`
   containing the generated packet. The next backend improvement is wiring this
   endpoint to configured model providers or coding-agent runners while keeping
   the same auditable operation boundary.

3. Agent providers and coding-agent runners are different concepts.

   `ProviderProfile` is currently OpenAI-compatible model transport. That is
   enough for local/OAI-compatible inference, but OpenAI Codex and Claude Code
   need to be modeled as coding-agent runners with command/MCP capabilities,
   write scopes, and visible worklogs. The web UI shows both lanes, but the
   backend should add a `x07.studio.agent_profile@0.1.0` schema.

4. XTAL lifecycle commands are binding-first, but long-running visibility must
   be explicit.

   `OpRecord` persists command details, and Studio now updates supervised
   `agent.run.*` records with stdout/stderr chunks while the command is still
   running. Studio also derives bounded `agent.event.*` records from those
   chunks for artifacts, diagnostics, write activity, and approval/policy
   requests. The next visibility layer is richer event sources from structured
   agent protocols instead of output-line classification alone.

5. Documentation is strong on agent quickstart but scattered for Studio.

   `x07/docs/getting-started/agent-quickstart.md`,
   `available-skills.md`, and the guides explain canonical loops, skills, and
   `x07 run`. Studio now compiles those into approved session doctrine,
   includes the selected refs and MCP tools in coding-agent handoff prompts, and
   renders the doctrine in the browser right rail. The remaining improvement is
   live doc resolution/snippet previews through `x07.doc_v1` or the docs index
   instead of showing refs only.

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
   appends a `running` record before execution, updates the same record with
   stdout/stderr chunks while the command is active, and then records final
   status and captured output. Raw chunks are classified into `agent.event.*`
   records for visible artifact, diagnostic, write, and approval/policy events.
   Human checkpoints are also explicit pending `agent.approval.*` records for
   approval-gated profiles. The remaining gap is structured protocol support for
   richer semantic approval prompts.

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

13. The visible worklog must fit the first viewport.

   The ImageGen concept puts the command stream in the same first viewport as
   the intent, lineage, agent, trust, and budget surfaces. A fresh browser
   render showed the operation log below the fold on a 1728x972 desktop
   viewport, which weakens the "all agent processes visible" requirement even
   though the underlying records existed. The browser shell now uses a bounded
   desktop grid with internal panel scrolling so the operation log remains
   visible without hiding the XTAL room controls. The e2e test now asserts that
   the operation log is in the viewport on initial load.

14. Browser e2e tests must not accidentally bind to a live daemon.

   The SvelteKit dev proxy used a fixed `http://127.0.0.1:7719` daemon origin.
   When a live Loom daemon was running during QA, the Playwright test opened the
   real connected surface instead of the deterministic demo projection and
   failed on the expected status text. The Vite config now reads
   `LOOM_DAEMON_ORIGIN`, and the Playwright web server points that origin at a
   closed local port so demo-mode e2e coverage stays hermetic even when a live
   daemon is active for separate rendered QA.

15. Trust review needs a queue, not only a long log.

   The operation log is canonical, but reviewers still need a small set of
   high-signal items when agent output, implementation sync, and verification
   records accumulate. The browser now derives a review queue from `OpRecord`s:
   agent artifacts, diagnostics, writes, approval requests, patchsets, verify
   evidence, and certify evidence become clickable signals that select the
   original operation in the inspector.

16. Patchsets need a file-level inspector before full diffs.

   `x07.patchset@0.1.0` is the canonical deterministic edit vehicle, but the
   browser previously only showed the artifact path. The operation inspector now
   recognizes embedded x07 patchset payloads, patchset artifact paths, and write
   roots, then renders affected files, JSON Patch operation counts, notes,
   review gates, and path risk. The remaining gap is reading artifact file
   contents from the daemon so Studio can show full before/after JSON Patch
   diffs when the patchset exists only as a file path.
