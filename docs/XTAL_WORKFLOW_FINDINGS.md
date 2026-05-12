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
   containing the generated packet. Provider-backed polish is now an opt-in
   enrichment lane that records model suggestions as bounded review evidence
   without bypassing the deterministic packet or human approval gate.

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
   requests. The handoff prompt now defines a structured
   `x07.studio.agent_event@0.1.0` JSONL protocol, and the daemon parses those
   lines before falling back to output-line classification.

5. Documentation is strong on agent quickstart but scattered for Studio.

   `x07/docs/getting-started/agent-quickstart.md`,
   `available-skills.md`, and the guides explain canonical loops, skills, and
   `x07 run`. Studio now compiles those into approved session doctrine,
   includes the selected refs and MCP tools in coding-agent handoff prompts, and
   renders the doctrine in the browser right rail. The right rail now resolves
   those refs through a bounded daemon docs-preview endpoint, showing file
   snippets or directory entries for `x07/docs/...` refs before agent handoff.

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
   `x07.studio.agent_profile@0.1.0`, generates session-contract handoffs, and
   runs approval-gated supervised commands through the daemon so API callers
   cannot bypass readiness policy.

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
   approval-gated profiles. Studio now also advertises and parses a structured
   `x07.studio.agent_event@0.1.0` JSONL protocol so Codex and Claude can emit
   approval, artifact, diagnostic, and write milestones without relying on
   fragile free-form text classification.

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

16. Patchsets need a file-level inspector before semantic diffs.

   `x07.patchset@0.1.0` is the canonical deterministic edit vehicle, but the
   browser previously only showed the artifact path. The operation inspector now
   recognizes embedded x07 patchset payloads, patchset artifact paths, and write
   roots, then renders affected files, JSON Patch operation counts, notes,
   review gates, and path risk. Studio now has a bounded daemon preview endpoint
   for recorded operation artifacts, so patchset artifact paths can become
   concrete patch entries without granting arbitrary filesystem reads. Recorded
   x07 patchsets now also get in-memory before/after JSON previews for bounded
   workspace targets, with target-level errors surfaced in the review row.

17. Docs-example seeding must not copy generated `.x07` state.

   A live Studio ladder run reached the complex `docs/examples/apps/x07-api-gateway`
   workflow and failed at `test.manifest` because the source example directory
   had stale local `.x07/deps` material. Studio copied that generated cache into
   the new workspace, so the test command hydrated `ext-obs@0.1.2` from stale
   package metadata and saw an `ext-net` version conflict. The seeder now skips
   `.x07` alongside `target`, `dist`, and `node_modules`, forcing new projects
   to hydrate dependencies from the current lockfile and registry metadata.

18. Crawler-shaped projects need explicit CLI-argument run bindings.

   `x07crawl` is not a spec-only XTAL starter: its documented flow passes
   process arguments after `--` and writes `out/crawl.json`. The browser intent
   parser already mapped crawler prompts to `crawl.plan`, but Studio previously
   had no seeded template or binding that could run the crawler replay flow.
   Studio now maps `crawl.plan` to `docs/examples/apps/x07crawl`, prepares
   `out/`, runs the crawler replay command with explicit sandbox fallback when
   needed, and bundles `dist/x07crawl`.

19. Full-stack app showcases need app-pipeline bindings, not web-ui-only bindings.

   `x07_atlas` exercises `x07-wasm app` profile validation, app build, trace
   replay, release packing, provenance, deploy planning, and SLO evaluation.
   Studio now maps `atlas.app` prompts to
   `docs/examples/wasm_showcases/x07_atlas` and runs the app-pipeline lane with
   fixed Atlas artifact paths so agents can create and verify the full-stack
   example from the same XTAL workflow control.

20. Workspace readiness needs a first-class surface, not only room-local panels.

   The phase plan calls for Studio to open on a Workspace Radar that shows XTAL
   readiness, active sessions, latest verify/certify state, incidents, provider
   state, and new-session actions. Before this pass those signals existed only
   as scattered right-rail, lifecycle, and intake details. The browser shell now
   has a compact radar band backed by the live session/op state, with direct
   Intent, Brownfield Extract, and Incident Improve actions that prepare the
   correct task type, input mode, prompt, and approval lane before session
   creation. Studio now also exposes `/v1/workspace/radar`, so connected
   browsers derive manifest presence, spec count, generated-test manifest state,
   latest verify/certify artifacts, and incident count directly from
   `arch/xtal/xtal.json`, `spec/**`, `gen/xtal/**`, `target/xtal/**`, and
   `.x07/studio/**` session state.

21. Verification failures need a repair theater, not only a selected log row.

   The operation inspector is useful for raw command evidence, but it does not
   summarize the XTAL repair decision for non-expert users. Studio now derives a
   Counterexample Theater from failed `OpRecord`s, diagnostics, repair artifacts,
   violation artifacts, and incident evidence. It names the failing clause,
   shows the smallest witness or command output, lists evidence artifacts, and
   keeps the safe route explicit: inspect the witness, run `xtal.repair`, and
   rerun verification before widening the spec. A live browser check against an
   empty workspace confirmed that failed `xtal.verify` output is classified into
   this repair surface and still selects the original operation for full audit.

22. Existing specs are an input source, not a post-intent shortcut.

   The product goal allows users to start from an initial plan or an existing
   spec. Before this pass, Studio only treated written plans, voice transcripts,
   and incident notes as first-class intent inputs, which meant a pasted
   `x07.x07spec` lost source provenance or had to be re-described as prose.
   Studio now has an `Existing Spec` input mode in the browser and daemon. The
   kernel records the raw spec as the intent source, derives the target module
   and entry from `module_id` plus the first operation, adds a spec-source
   witness, and still routes the result through the same human approval gate. A
   live connected browser check confirmed the spec control, textarea, revision
   input, room select, tabs, daemon session record, and zero console issues.

23. Complex examples need visible world and budget gates before agent execution.

   The phase plan calls out World Map, Trust Border, and Budget Heatmap overlays,
   but the browser only had a generic world-evidence card and a synthetic credit
   meter. That was too weak for projects like `x07-api-gateway`, `x07dbguard`,
   and `x07_atlas`, where the important review question is whether the agent is
   crossing from solve-pure into solve-rr, sandbox/run-os, WASM app, release,
   provenance, or SLO/budget lanes. Studio now derives a World / Budget Guard
   from the selected example brief, session contract, and operation records. The
   panel keeps deterministic solve-pure visible, flags capability widening, shows
   budget evidence requirements, and lists review gates before supervised agent
   execution.

24. Revision requests must block stale approval.

   The UI had `Request Changes`, but after a revision was appended the same
   already-polished intent could still be approved. That violates the product
   requirement that the coding agent keeps improving the plan until human
   approval. Studio now treats the revision state as an approval blocker:
   `Approve Spec` stays disabled until `Polish Intent` runs again, and the
   approval ledger explains why realization is blocked.
   Intent room shows an approval ledger with the input source, polish step,
   revision notes, human decision, and write-contract lock.

25. Agent handoffs need the same execution boundaries that humans see.

   The right rail now shows world and budget gates, but the daemon-generated
   Codex/Claude handoff prompt still only listed generic guardrails, verbs,
   tools, write roots, contract, and intent. That left complex projects at risk
   of asking an agent to infer solve-rr, sandbox/run-os, WASM app,
   release/provenance, or SLO/budget boundaries from prose. The handoff prompt
   now includes an explicit Execution Boundary section: `x07 run` is the default
   execution front door, solve-pure is the default lane, and any detected
   capability or release/budget widening is named as an approval-gated surface.

26. Brownfield extract must run before spec approval.

   The Workspace Radar prepared a Brownfield Extract prompt and task type, and
   the binding catalog already exposed `spec.extract`, but the approval path
   treated brownfield like a normal new-behavior session. That let humans approve
   without ever extracting current implementation behavior. Studio now runs
   `spec.extract` during the `spec_draft` phase for `brownfield_extract`
   sessions and only locks the session contract after that operation succeeds.

27. Incident notes must become canonical XTAL incident artifacts before improve.

   `x07 xtal ingest` does not accept arbitrary prose; it accepts XTAL violation
   bundles, contract repros, or recovery-event JSONL. The Studio incident form
   previously captured an incident note but pointed `xtal.ingest` and
   `xtal.improve` at a prose placeholder, so the visible runtime-improve lane
   could not be run by the canonical x07 tools. Studio now persists each manual
   incident note as a session-scoped `x07.xtal.violation@0.1.0` bundle with a
   matching `x07.contract.repro@0.1.0`, ensures an `arch/xtal/xtal.json`
   manifest exists when needed, and runs `xtal.ingest --normalize-only` followed
   by `xtal.improve` from the approval/run path.

28. Platform delivery must use the current x07lp deployment surface.

   Studio had shallow x07-platform bindings for hosted release query, hosted
   rollback, and rollout status, but the command shapes had drifted from the
   current platform driver. The platform surface now exposes `release-query`,
   `release-rollback`, and local `accept`/`run`/`query`/`status`/incident
   bindings. Atlas now uses a typed delivery lane instead of a static command
   chain: `accept` returns the deployment execution id that the following
   `run`, `query`, and status commands need.

29. Platform paths need two representations in Studio.

   The direct `x07-platform/scripts/x07lp-driver` resolves relative inputs from
   the platform checkout, while Studio runs inside the generated project
   workspace. Local platform delivery therefore needs absolute command arguments
   for pack, plan, metrics, and state paths, but relative artifacts in the
   Studio session log so artifact preview and trust review remain scoped to the
   user project. Studio also resolves a sibling
   `x07-platform/scripts/x07lp-driver` when `x07lp` is not installed on `PATH`,
   because that is the common local multi-repo development layout.

30. The design reference must be a product surface, not a landing page.

   A fresh ImageGen concept is committed at
   `docs/design/xtal-studio-ui-mockup.png`. The useful parts are the lifecycle
   radar, lineage graph, plan approval loop, visible Codex/Claude worklog,
   trust deltas, and bottom canonical command lane. Studio now mirrors the
   command-lane and radar ideas in code-native controls instead of shipping the
   mockup as a static screenshot.

31. MCP doctrine should be selectable like the other lifecycle rooms.

   The earlier browser shell exposed MCP tools and canonical x07 docs only in
   the right rail. That made the docs/tool contract feel secondary even though
   XTAL relies on agents starting from a compiled doctrine. Studio now has an
   `MCP` room entry that focuses the Session Doctrine panel, keeping
   `x07.search_v1`, `x07.context_pack_v1`, `x07.exec_v1`, and the canonical
   `x07/docs/getting-started/**` references visible before agent handoff.

32. End-to-end automation needs a runbook before execution.

   `Approve and Run` was backed by canonical bindings and a visible operation
   log, but non-expert users still had to infer what would happen after
   approval. Studio now derives an XTAL Automation Plan from the selected
   docs-example brief, approval state, session contract, and operation log. It
   shows plan polish, human approval, project scaffold, spec/test/incident
   steps, implementation sync, canonical commands, expected artifacts, and
   blocked/ready/running/done/failed state before and after execution. This
   keeps automatic project creation explainable without weakening the XTAL
   approval gate.

33. External agent prompts need the same runbook as the UI.

   The browser now shows the approval-gated automation plan, but Codex and
   Claude handoff prompts still only described guardrails, boundaries, tools,
   write roots, and the required loop. Studio now renders an Automation Runbook
   section into daemon-generated handoff prompts. It names the human approval
   gate, project scaffold, spec/test or incident steps, implementation sync,
   verification, repair/certification gates, and WASM/release/provenance/SLO
   commands when the session implies those surfaces. This keeps supervised
   coding agents aligned with the same plan that humans review.

34. Agent readiness must be enforced by Loom, not only the browser.

   The browser disabled run controls for missing or disabled Codex/Claude
   profiles, but direct daemon API calls could still generate handoffs or start
   supervised runs. Loom now rejects disabled agent profiles for handoff, plan,
   and execute operations, and `mode: "execute"` checks the configured command
   before appending a supervised `agent.run.*` record. This keeps the UI
   readiness panel and daemon policy aligned.

35. Provider polish should enrich intent review, not replace it.

   Studio's deterministic intent packet is the baseline that keeps the XTAL
   flow auditable. The new provider-polish lane is therefore opt-in: it sends
   the deterministic packet and revision notes to a configured
   OpenAI-compatible provider, accepts only bounded review metadata, and records
   the provider report under `intent.formalize`. Missing, disabled, failing, or
   non-JSON providers fall back to the deterministic packet with the failure
   recorded as evidence instead of silently changing the workflow.

36. Standalone desktop CI must prove onboarding, not only compilation.

   The desktop matrix already built native shells, the Svelte web app, and a
   bundled `x07-wasm`, but a smooth first-run experience also depends on
   `defaults.env`, launcher scripts, the archive layout, and component bootstrap
   discovery. The CI workflow now validates those surfaces after packaging:
   manifest, static web app, zip contents, first-run workspace/daemon/web
   defaults, and the bundled `x07-wasm` status reported by the copied bootstrap
   script.

37. Browser E2E needs one real daemon path beside the hermetic demo path.

   The default Playwright test intentionally points the web proxy at a closed
   port so live local daemons cannot make demo-mode assertions flaky. That left
   the browser-to-daemon path covered by unit and Rust tests, but not by a
   rendered connected UI flow. Studio now has `npm run e2e:connected`, which
   starts a real Loom daemon against a temp workspace, supplies deterministic
   local `x07`, `x07-wasm`, and `x07lp` shims, and verifies a simple XTAL
   session can be created, polished, approved, run, and extended through the
   canonical binding selector without entering demo mode.

38. First-run readiness needs an action plan, not only badges.

   The health endpoint already reports defaults and component readiness, but
   the browser only displayed compact status cards. That was enough for expert
   users, but not for an end user starting from a standalone bundle or a fresh
   sibling checkout. Studio now derives an onboarding setup plan from the same
   health payload: first-run defaults, the bootstrap command, resolved runtime
   component paths, missing required components, and optional Codex/Claude
   agent setup are visible before the user creates a project.

39. Standalone launch defaults must reflect the running process.

   The packaged web launcher used fixed local ports from `defaults.env`, and the
   daemon health response always reported `127.0.0.1:7719`. If another local
   service already owned that port, first-run launch could fail or show stale
   setup guidance even after the launcher recovered. The launcher now selects
   free daemon and web ports when defaults are busy, exports those choices to the
   daemon process, and daemon health reports the runtime defaults back to the
   browser onboarding plan.

40. Standalone CI should launch the artifact, not only inspect it.

   The package validator proved the manifest, scripts, web build, and bundled
   components existed, but it did not start the launcher as an end user would.
   The desktop matrix now occupies the default daemon/web ports, starts the
   packaged web launcher, fetches the built app, and verifies `/v1/health`
   reports the runtime fallback daemon address.

41. Native desktop onboarding must not lag behind the web shell.

   The browser setup panel showed first-run defaults, bootstrap command, missing
   required components, and optional Codex/Claude setup, but the egui shell only
   showed compact component badges. The native shell now derives the same setup
   plan from daemon health, and its embedded daemon exports the actual random
   loopback address before serving so `/v1/health` no longer reports the static
   default port.

42. Connected E2E must cover the complex workflow, not only the starter path.

   The rendered demo test already drove the project form from simple examples
   through `x07_atlas`, but the real-daemon browser test only exercised the toy
   sorter. That left the x07-wasm, local platform, and supervised-agent surfaces
   dependent on unit tests and demo-mode assumptions. The connected E2E harness
   now provides deterministic `x07-wasm`, `x07lp`, Codex, and Claude shims and
   proves a simple-to-Atlas project ladder can be created through the form,
   revised, approved, handed to Claude Code, executed through the x07 Atlas
   workflow, and surfaced in trust review without entering demo mode.

43. Voice must become an intent witness, not a shortcut to code generation.

   The UI already had a `Voice Transcript` input mode, but it only changed the
   intent source label. That made spoken programming look supported while still
   requiring a pasted transcript. The browser now exposes a Web Speech capture
   control that appends final transcript segments as `Voice witness:` lines in
   the same initial-plan textarea, keeps the user in the approval-gated intent
   path, and falls back to paste-transcript guidance when speech capture is not
   available. Studio also classifies the draft input before polish so the user
   can see whether the text is being treated as desired behavior, forbidden
   behavior, policy requirement, or incident evidence. The Playwright test
   injects a deterministic `SpeechRecognition` shim so CI verifies that a
   spoken workflow-graph witness reaches spec review before approval.

44. Non-experts need a prompt-to-artifact audit, not only a command log.

   The worklog and automation plan are useful for agents and reviewers, but a
   user starting from natural language still needs a compact answer to "what
   proof points are covered now?" Studio now derives a Prompt-to-Artifact audit
   from the selected project brief, session approval state, and `OpRecord`s. It
   maps initial plan/spec capture, human approval, project scaffold, spec/tests,
   implementation realization, verification, visible agent work, and
   trust/platform evidence to concrete artifacts and operation records. Each
   audit row can select its source operation, so the summary remains grounded in
   the canonical log instead of becoming a second source of truth.

45. Platform integration needs an Ops bridge, not scattered release rows.

   Studio already had the canonical `x07-wasm` and `x07lp` bindings needed for
   Atlas-shaped projects, and trust review could surface deploy and SLO signals.
   That still forced non-expert users to assemble the platform story from the
   worklog, audit panel, and review queue. The browser now derives an x07
   Platform bridge from the same `OpRecord`s: app package verification,
   provenance, deploy plan, local platform delivery, SLO/budget evidence, and
   incident feedback are separate clickable gates. This keeps the best
   integration path explicit: `x07-wasm app` evidence feeds provenance and
   deploy planning, `x07lp` owns local delivery state, and incidents return to
   `xtal.ingest` / `xtal.improve` instead of becoming ad hoc follow-up work.

46. Agent handoffs must be reviewable before execution.

   Codex and Claude Code integration already generated session-contract
   handoff prompts and supervised commands, but the browser mostly surfaced the
   saved prompt path. That hid the actual operating doctrine from the human who
   approves the external agent run. The Agents room now derives a handoff
   contract view from the same handoff response or recorded `agent.*` operation:
   command, prompt path, approval gate, execution boundaries, automation
   runbook, allowed verbs, MCP tools, write roots, prompt excerpt, and
   `x07.studio.agent_event@0.1.0` protocol are visible before execution. This
   keeps Codex/Claude integration aligned with XTAL's rule that agents act
   through finite, reviewable lifecycle verbs instead of an opaque chat prompt.

47. Non-expert users need a focused workflow view before the full audit wall.

   The browser had accumulated the right XTAL evidence surfaces, but the
   default screen still behaved like a dense reviewer console. That works for
   experts, but it slows the target user who starts with a plan and needs the
   next legal action, visible agent work, and canonical command lane without
   reading every trust, budget, and MCP panel at once. Studio now opens in a
   focused room layout, keeps the operation log in the first workflow view,
   adds a `Details` toggle for the full audit surface, and gives the Realize and
   Verify rooms compact first-class panels. The connected browser test now
   proves the same focused/detail controls work against a real Loom daemon and
   the simple-to-Atlas XTAL workflow.

48. Codex and Claude Code must both be proven as supervised execution lanes.

   Studio already rendered readiness cards for OpenAI Codex and Claude Code,
   but the connected browser test only executed the Claude path. That left a
   coverage gap in the requirement that both coding agents integrate with the
   same handoff, approval, supervised command, worklog, and structured
   `x07.studio.agent_event@0.1.0` protocol. The connected test now drives an
   OpenAI Codex handoff through plan, approval, execute, artifact event, and
   Codex worklog filtering before repeating the Claude Code flow.

49. Coding-agent contracts need a machine-readable process surface.

   The handoff prompt named allowed verbs, write roots, MCP tools, and approval
   gates, but the launched Codex or Claude process only received a markdown
   file path. That invites each coding agent wrapper to parse prose before it
   can enforce Studio's XTAL contract. Loom now exports `X07_STUDIO_*`
   environment variables to supervised agent commands: session id, agent id,
   handoff path, allowed verbs, MCP tools, write roots, approval mode, and the
   structured event schema. This is still not a full OS path sandbox, but it
   gives agent wrappers and future launchers a deterministic contract surface.

50. Supervised agents need write-root evidence, not only write-root advice.

   The `X07_STUDIO_WRITE_ROOTS` environment variable made the approved write
   contract machine-readable, but a misbehaving or mismatched agent could still
   write outside those roots and exit successfully. Loom now snapshots bounded
   workspace source/config files before and after supervised agent execution.
   If the command creates, modifies, or deletes files outside the approved
   roots, Studio marks the `agent.run.*` operation failed and records a
   `x07.studio.agent_write_audit@0.1.0` payload with created, modified,
   deleted, and violating paths. The browser review queue and operation
   inspector now surface that audit as first-class evidence. This does not
   replace a future OS-level sandbox, but it prevents unauthorized writes from
   becoming silent success.

51. Write-root audits need connected browser proof.

   Unit coverage proved the write-root audit parser and the Loom kernel failure
   path, but the real XTAL workflow target is a human reviewing the browser
   surface while the daemon supervises an agent. The connected E2E now installs
   a temporary `Write Audit Agent`, runs it from Studio, intentionally writes
   outside `src/`, and verifies the browser shows the failed `agent.run.*`
   record, `Write-root audit` review signal, allowed roots, and violating path.

52. Custom agent profiles must not hide the built-in Codex and Claude lanes.

   Adding the connected write-audit agent exposed a profile-loading bug: once a
   custom agent was saved, Loom returned only saved profiles and the default
   OpenAI Codex and Claude Code profiles disappeared from the browser selector.
   Loom now merges saved profiles with the built-in profiles, letting saved
   profiles override by id while preserving Codex and Claude Code when
   additional local runners are configured.

53. Graph overlays need first-class modes, not hidden right-rail panels.

   The browser already derived world, trust, and budget evidence, but the XTAL
   graph still looked like a static lineage diagram. That hid the phase-plan
   overlays from the place where users inspect the whole workflow. Studio now
   adds Lineage, World Map, Trust Border, and Budget Heatmap modes directly to
   the graph panel. Each mode highlights the relevant lifecycle nodes and shows
   a compact evidence summary beside the graph, while the browser workflow test
   proves the overlay controls and status feedback.

54. Patch review must explain meaning, not only file paths.

   File-level patch review and before/after JSON previews made deterministic
   x07 patchsets visible, but a non-expert still had to read raw x07AST to know
   whether a patch touched an export, implementation body, spec contract, or
   policy boundary. The browser now derives semantic patch rows from JSON Patch
   operations, enriches them with bounded before/after preview values when
   available, and renders a side-by-side semantic diff inside the operation
   inspector. The demo workflow test now proves an implementation sync patch
   exposes that semantic review surface before the user continues reviewing
   trust evidence.

55. Spoken intent needs confidence review before approval.

   Voice capture already entered the same approval-gated intent path as written
   input, but the browser treated every final transcript as equally reliable.
   That is too weak for non-expert users because a speech recognizer can turn a
   requirement or forbidden behavior into the wrong witness. Studio now records
   Web Speech confidence for final segments when the browser provides it,
   exposes language and confidence-gate controls, and marks low-confidence or
   unavailable-confidence transcripts for review in the captured witness list.
   The browser voice test now proves a low-confidence spoken workflow witness
   remains visible before spec approval instead of silently becoming trusted
   input.

56. Provider polish needs capability gates, not only a profile id.

   Provider-backed intent polish was opt-in and recorded as review evidence,
   but the browser mostly showed a checkbox plus raw provider profile id. That
   made it hard for a human to know whether a provider had proven `/models`,
   `/responses`, `/chat/completions`, tool calls, JSON schema, streaming, or
   trust tier before model suggestions entered the intent review. Studio now
   lists configured OpenAI-compatible profiles, runs the existing provider probe
   endpoint from the browser, and renders capability gates beside the Codex and
   Claude Code lanes. The browser workflow test now proves the probe gate is
   visible and that provider-backed polish stays reviewable rather than being a
   hidden prompt enhancement.

57. Proof caching needs a visible dependency ledger before a compiler cache.

   The phase plan calls for proof and trust evidence to be reusable only when
   the spec, implementation, proof policy, verify artifact, and certification
   context still match. Studio had verify and certify operation records, but no
   single surface that explained what a future proof cache key would depend on.
   The Verify room now renders a proof-cache readiness ledger with a
   deterministic key preview and explicit spec, implementation, proof-policy,
   verify-artifact, and certification rows. The browser test proves the ledger
   appears after a simple XTAL run while still labeling the cache as a preview,
   not a persisted compiler-backed proof cache.

58. Verify-room proof controls must affect the canonical command.

   The phase plan names proof policy, world selection, and verification bounds
   as Verify-room controls. Studio showed verify evidence and proof-cache
   dependencies, but the browser had no way to choose `balanced` versus
   `strict`, allow an OS-capable world, or set proof bounds before running the
   XTAL workflow. That would force users back to a terminal for one of the main
   proof decisions. Studio now carries bounded verify variables through the web
   API and Loom daemon, validates them, and renders them as real
   `x07 xtal verify` flags in the operation record. The browser workflow test
   proves the command preview, proof-cache key, and operation log all reflect a
   strict proof run with explicit bounds.
