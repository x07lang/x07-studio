# v0.1 status

## Wired now

- real x07 / x07-wasm / x07lp command execution through `loom-adapters::x07_cli`
- machine-readable report capture for x07 and x07-wasm
- structured stdout/stderr capture for x07-platform bindings
- streaming stdout/stderr updates for supervised coding-agent commands
- semantic `agent.event.*` records derived from supervised-agent artifacts, diagnostics, writes, and approval/policy requests
- browser trust review queue derived from artifacts, diagnostics, writes, patchsets, verify evidence, and certify evidence
- prompt-to-artifact audit that maps user input, approval, scaffold, specs/tests, implementation, verification, visible agent work, and trust/platform evidence to concrete operations and artifacts
- browser proof-cache readiness ledger that previews cache keys and names the spec, implementation, proof-policy, verify artifact, and certification dependencies
- browser XTAL verify controls for proof policy, OS-world override, and proof bounds that render into validated `x07 xtal verify` flags
- browser Verify evidence board that projects `x07.xtal.verify_summary@0.1.0` into precheck, coverage, proof, generated-test, diagnostic, entry, and artifact rows
- browser XTAL repair controls for entrypoint, strategy, write mode, candidate bounds, semantic depth, and non-stub edit review that render into validated `x07 xtal repair` flags
- browser XTAL certify controls for spec directory, entry scope, all-entry scope, and precheck policy that render into validated `x07 xtal certify` flags
- browser Certify evidence board that projects `x07.xtal.certify_summary@0.1.0` into project digest, precheck, review-gate, certificate, trust-report, review-diff, and bundle rows
- browser Certify bundle preview that projects `x07.xtal.cert_bundle@0.1.0` into entry, file, external-file, spec-digest, example-digest, byte-count, and digest inventory rows
- browser focused/detail modes that keep the current XTAL room, visible agent worklog, and canonical command lane in the first workflow view while preserving the full audit surface behind an explicit Details toggle
- browser graph overlay modes for Lineage, World Map, Trust Border, and Budget Heatmap on the XTAL graph panel
- x07 Platform bridge that traces app packaging, provenance, deploy planning, local x07lp delivery, SLO/budget checks, and runtime feedback to canonical operation evidence
- visual patch review in the operation inspector for x07 patchset payloads, patchset artifacts, write roots, review gates, path risk, and before/after JSON previews
- semantic patch review rows that classify x07 patch operations by contract, implementation, export, policy, and evidence impact
- world/budget guard for solve-rr, sandbox/run-os, WASM app, release/provenance, and budget widening surfaces
- bounded daemon artifact preview for recorded operation artifacts, including in-memory JSON Patch previews for x07 patchsets
- binding coverage for XTAL authoring, test generation, implementation sync, verify/repair/certify/ingest/improve, core checks, app, web-ui, device, workload, topology, deploy-plan, SLO, provenance, and selected platform query/control reads
- seeded docs-example workflows for workflow graph, state-machine contracts, API gateway, x07crawl, x07dbguard, and x07 Atlas projects
- clickable automation-plan steps that link completed approval-to-artifact runbook rows to their operation evidence, including seeded `project.seed.*` project setup
- template-aware automation plans that keep XTAL-pure scaffold/test/impl rows out of docs-seeded workflows such as x07 Atlas
- brownfield approval path that runs `spec.extract` before session contract lock
- approval ledger that blocks stale approvals after human revision requests until the agent repolishes intent
- daemon-owned revision request operation records (`intent.revision.request`) that keep requested changes visible before repolish
- approval/run controls stay blocked until a polished intent packet is visible for human review
- approval preview revision review that shows whether requested changes are pending repolish or visible in the polished intent packet
- session doctrine surfacing for canonical x07 docs, MCP tools, allowed verbs, and handoff prompt context
- reviewable Codex/Claude handoff contract panel showing command, prompt path, approval gate, execution boundaries, automation runbook, allowed verbs, MCP tools, write roots, prompt excerpt, and agent event protocol
- browser agent flight recorder that summarizes the selected Codex/Claude handoff, launch plan, human checkpoint, supervised run, agent events, and write-root audit in one clickable timeline
- focused-mode agent actions preserve the Agents room across handoff, launch-plan, approval, and run session refreshes
- bounded session docs preview for `x07/docs/...` refs, including file snippets and directory indexes
- agent handoff execution-boundary prompts for x07 run, solve-rr, sandbox/run-os, WASM app, release/provenance, and SLO/budget gates
- MCP HTTP transport with initialize + session header handling
- MCP stdio transport with newline-delimited JSON-RPC
- daemon-owned `intent.formalize` endpoint for written plans, voice transcripts, existing specs, incident notes, revision notes, and visible intent operation records
- browser voice transcript capture that appends spoken witnesses into the same approval-gated intent path, with paste-transcript fallback when Web Speech is unavailable
- browser transcript confidence review for Web Speech witnesses, including language selection, confidence gate, and low-confidence review badges
- draft witness preview that classifies raw written/spoken/spec/incident input before polish as desired behavior, forbidden behavior, policy requirement, or incident evidence
- opt-in provider-backed intent polish that records model suggestions as review evidence while keeping deterministic intent generation as the fallback
- browser provider capability gates for OpenAI-compatible profiles, including model catalog, intent-polish API, tool calls, JSON schema, streaming, and trust-tier review
- daemon health reports onboarding defaults and runtime component readiness for `x07`, `x07-wasm`, `x07lp`, and local agent CLIs
- browser and native egui onboarding plans render first-run defaults, bootstrap command, resolved component sources, and required/optional setup work from the daemon health report
- daemon-side coding-agent readiness checks reject disabled profiles and missing execute commands even when clients bypass the browser controls
- saved custom agent profiles merge with the built-in OpenAI Codex and Claude Code lanes instead of replacing them
- connected browser E2E proves both OpenAI Codex and Claude Code supervised handoffs through plan, approval, execute, structured agent event capture, and worklog filtering
- supervised agent commands receive `X07_STUDIO_*` contract environment variables for session id, agent id, handoff path, allowed verbs, MCP tools, write roots, approval mode, and event schema
- supervised agent commands run with a post-execution workspace write-root audit that fails the run when source/config files change outside the approved roots
- browser review queue and operation inspector surface failed agent write-root audits as first-class review evidence
- connected browser E2E proves a supervised agent write-root violation becomes a failed run plus visible review signal and inspector evidence
- standalone packaging scripts assemble the daemon, native desktop shell, Forge shell, and static Svelte web app into a portable bundle
- standalone launcher refreshes setup defaults, selects free local daemon/web ports, and reports the actual runtime addresses through daemon health
- native desktop shell runs packaged first-run component bootstrap before starting its embedded daemon, with skip/detect-only flags for controlled onboarding
- standalone CI validates bundle manifest, static web app, launcher scripts, first-run defaults, bundled `x07-wasm` wiring, and live launcher startup behind occupied default ports
- connected browser E2E starts a real Loom daemon and runs the web app through simple XTAL and Atlas-level x07-wasm/platform sessions without falling back to demo mode
- OpenAI-compatible provider probing through `/models`, `/responses`, and `/chat/completions`
- Axum daemon routes for sessions, bindings, providers, and MCP connections
- egui GUI shell and ratatui Forge shell over the daemon API
- egui GUI shell starts an embedded local daemon by default for standalone desktop use and reports that runtime daemon address through health

## Still intentionally thin in v0.1

- GUI exposes HTTP MCP first; stdio MCP is available through the daemon API already
- provider probing is bounded and capability-oriented; it now surfaces readiness gates but is still not a full quality benchmark suite
- session execution policy is enforced by the reducer, canonical binding catalog, approval gates, supervised agent contract environment, and post-run write-root audits; this is still not an OS-level path sandbox
- the v0.1 shells expose lifecycle controls, graph overlays, artifact logs, a compact trust review queue, artifact-backed patchset previews, semantic patch review rows, path-level before/after visual patch review, and browser speech transcript capture with confidence review; richer provider and STT backend configuration remains a later UI layer
- no compiler-backed persisted proof cache yet
- no persisted audio capture or local STT model selection yet

## Validation done here

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- focused tests cover reducer transitions, unknown-session handling, binding catalog rendering, provider probing, MCP tool parsing, and filesystem persistence
