# Stress-pass findings (Cycle 7)

Live notes from running the real x07 0.2.10 + claude 2.1.140 + codex 0.130.0 toolchain through Studio. Each entry: severity / observation / evidence / fix or open status.

Severity legend:
- **BLOCK** — must fix before production.
- **FIX** — should fix before public beta.
- **NOTE** — visual/UX nit; document.

## Index

| # | Severity | Title | Scenario |
|---|---|---|---|
| F1 | **FIXED** | HealthRow MIGRATE label reads "schema → 0.5" when `from_schema` is null | 1 |
| F2 | **FIXED** | TrustCard pending stays "pending" for >1 min on a fresh real workspace | 1 |
| F3 | **FIXED** | Daemon HTTP server returns "Empty reply" after autopilot kicks off real claude/codex (resolved by yielding-autopilot refactor; lock released around AutoClarify subprocess select-loop) | 1 |
| F4 | **FIXED** | Real `x07 verify` proof-support warnings surface in TrustCard proof-support notes | 1 |
| F5 | **DOCUMENTED** | Real x07 picks discovery path from a sibling debug build, not `~/.x07/bin/` | infra |
| F6 | **FIXED** | Intent heuristic doesn't recognize text-normalization / casefold / unicode prompts; falls through to generic `app.main.run_v1` with no semantic guidance, leaving the role-pipeline reviewer in a stall loop | 5 |
| F7 | **FIXED** | Architect role emits a dummy "spec confirmed" log without enriching the scaffolded spec; coder gets empty `requires/ensures/doc` and produces identity passthrough (deterministic floor — `doc` enrichment from archetype semantics) | 5 |
| F8 | **FIXED** | `claude -p` variadic flags (`--add-dir`, `--allowedTools`) swallow the trailing prompt positional; architect-enrich subprocess produced 0 bytes and timed out | tier2 |
| F9 | **FIXED** | claude `--output-format stream-json` wraps the agent's text inside assistant/result envelopes; top-level `agent_event` parser missed the embedded `spec_enrichment` line | tier2 |
| F10 | **FIXED** | `architect_enrich_after_scaffold` was only wired into the fresh-scaffold branch; the xtal-pure template auto-installs `spec/toy.sorter.x07spec.json` so the else branch was taken and no enrichment ran | tier1.5 |
| F11 | **FIXED** | `architect_enrich_after_scaffold` wrote `serde_json::to_string_pretty`, which fails x07's `WXTAL_SPEC_NONCANONICAL_JSON` gate; downstream `xtal.verify` rejected the spec | tier1.5 |
| F12 | **FIXED** | Build-pipeline scaffolds the impl as `["bytes.empty"]`; any non-trivial `ensures` predicate produced a counterexample. Tier-1.5b lands: predicates promote **after** `try_template_synthesis` / coder writes a real impl, mirrored into both spec and impl files | tier1.5b |
| F13 | **FIXED** | Scenario 2 CSV repair now exercises real repair: seeded CSV examples make `xtal.verify` fail, `xtal.repair` runs, and autopilot pauses cleanly at `realize_stalled` | 2 |
| F14 | **FIXED** | Scenario 3 trust widening now records real `run-os` world, `os-time` capability, amber posture, and ladder solve-default loss | 3 |
| F15 | **FIXED** | Scenario 4 PBT now runs a target manifest, captures a counterexample, and locks it as a deterministic regression | 4 |
| F16 | **FIXED** | Role-overridden builds now still emit summary/lint evidence so autopilot can enter the role pipeline on scaffold-only output | 5 |
| F17 | **FIXED** | Text normalize/casefold targets get a dependency-backed Unicode implementation floor with `ext-unicode-rs@0.1.5` | 5 |
| F18 | **FIXED** | Streaming agent timeouts kill child processes, preserve partial output, and continue to bounded fallback | 5 |

## F1 — MIGRATE pill reads "schema → 0.5" when nothing exists yet

**Severity:** NOTE.

**Observed:** On a fresh real-toolchain workspace, the HealthRow renders `MIGRATE schema → 0.5` even though no `x07.json` existed at probe time. The health snapshot endpoint returns `from_schema: null, to_schema: "0.5"`.

**Evidence:** `target/stress-pass/scenario-1-text-utils/screenshots/01-real-landing.png`, plus health snapshot:

```json
"migrate": {
  "needs_migration": true,
  "from_schema": null,
  "to_schema": "0.5"
}
```

**Fix sketch:** `HealthRow.svelte` should render the value as `init → 0.5` (or just `pending`) when `from_schema` is null. The current `${from ?? 'schema'} → ${to}` expression masks the null/init case.

**Status:** **FIXED** in the pre-production readiness pass. `HealthRow.svelte` now renders the fresh-workspace case as `init → 0.5`, renders existing migrations as `<from> → <to>`, and hides the migrate pill when no migration is needed. `HealthRow.test.ts` covers all three cases.

---

## F2 — TrustCard sticks on "pending" while real autopilot is mid-flight

**Severity:** NOTE.

**Observed:** After clicking the recipe and starting autopilot, the TrustCard remains "pending" while real x07 is doing 20-60 seconds of scaffolding/verify work. The Process Lane should show activity in that window, but a posture-pending TrustCard reads as "stuck" to a user who doesn't know the difference.

**Evidence:** Network log shows `/v1/sessions/{id}/trust/posture` returning 404 until the first `posture.captured` op lands.

**Fix sketch:** When session phase is past `intent_drafting` but no posture has been captured yet, the TrustCard should show an animated "Computing trust posture…" state instead of the same idle "pending" copy used at zero-session.

**Status:** **FIXED** in the pre-production readiness pass. `TrustCard.svelte` now accepts `isComputing`, renders `Computing trust posture...` while a session is past intent drafting but posture is still absent, and respects `prefers-reduced-motion`. `TrustCard.test.ts` covers captured, computing, and idle states.

---

## F3 — Daemon HTTP returns "Empty reply" mid-autopilot (BLOCK)

**Severity:** **BLOCK — production-critical.**

**Observed:** ~30-60 seconds after autopilot fires real claude/codex subprocesses, every GET to the daemon (`/v1/health`, `/v1/sessions`, `/v1/sessions/{id}`) returns curl `* Empty reply from server` — TCP connection accepted, request sent, daemon closes the socket with no bytes written. The daemon process stays alive (`status=S`, listening on the port), but no response is produced. SSE stream connections accumulate as `ESTABLISHED` but never receive frames.

The work *does* progress on disk: AGENT.md gets written, spec gets drafted, impl gets implemented, `target/xtal/xtal.verify.diag.json` lands with `ok: true`. From the file system, the canonical loop completed. From the UI, the user sees a frozen page.

**Evidence:**

- `target/stress-pass/scenario-1-text-utils/screenshots/02-frozen-ui.png` — UI frozen mid-flow.
- `target/stress-pass/scenario-1-text-utils/real-verify-diag.json` — proves real x07 verify finished successfully.
- curl probe log:

  ```
  > GET /v1/sessions HTTP/1.1
  > Host: 127.0.0.1:7747
  > User-Agent: curl/8.7.1
  > Accept: */*
  >
  * Request completely sent off
  * Empty reply from server
  * Closing connection
  ```

- 4 ESTABLISHED connections piled up on `lsof -p <daemon>`:

  ```
  loom-daem 5915 webik  9u  TCP localhost:7747 (LISTEN)
  loom-daem 5915 webik 10u  TCP localhost:7747->localhost:51530 (ESTABLISHED)
  loom-daem 5915 webik 11u  TCP localhost:7747->localhost:51532 (ESTABLISHED)
  loom-daem 5915 webik 12u  TCP localhost:7747->localhost:51579 (ESTABLISHED)
  loom-daem 5915 webik 13u  TCP localhost:7747->localhost:51534 (ESTABLISHED)
  ```

**Hypothesis:** Either (a) a tokio runtime deadlock between the agent-spawn future and the session-state mutex, or (b) connections accumulate because the SSE handler doesn't drop on client disconnect, exhausting the connection pool, or (c) a `tokio::sync::Mutex` is being held across an `await` on an agent process.

**Reproduces every time** with real claude/codex. Never seen with the fake toolchain (which returns instantly from each subprocess).

**Diagnostic next step:** add tokio-console instrumentation + log every mutex acquire/release in `kernel.rs::run_role_pipeline` and `kernel.rs::execute_agent_command_streaming`. Re-run scenario 1 to capture the trace.

**Status:** **FIXED** (commit on top of 915f75a). Root cause was the daemon holding `state.kernel.lock().await` for the entire duration of `kernel.run_autopilot(...)`, including the inner subprocess select-loop. The fix introduces `run_autopilot_yielding` which takes `Arc<Mutex<WorkspaceKernel>>` and acquires/releases the lock per autopilot iteration, and for the `AutoClarify` step specifically releases the lock around the subprocess select-loop (mirroring the pattern the `run_intent_clarify` HTTP handler already used). The `start_autopilot` handler now calls `WorkspaceKernel::run_autopilot_yielding(state.kernel.clone(), ...)` instead of `kernel.run_autopilot(...).await` under a held lock.

**Verification:** with real `claude` running, 5/5 `curl --max-time 3 /v1/health` probes returned HTTP 200 in <2ms (screenshot: `target/stress-pass/scenario-1-text-utils/screenshots/03-f3-fix-mid-clarify.png` — page rendered Process Lane "Clarify assumptions: running" while claude was active and full session state was fetchable).

**Remaining work (deferred F3-followup):** `AutoBuild`, `AutoRealize`, `AutoClimb` still acquire the lock for the duration of their inner `run_build_pipeline` / `run_role_pipeline` / `climb_rung` calls. Those inner methods are `&mut self` and await `x07` CLI subprocesses while holding `&mut`, which means the lock is still held inside those steps. The yielding refactor releases the lock *between* steps and within the long-running AutoClarify select-loop; per-substep release inside the build/realize/climb pipelines is the remaining work for full responsiveness during scenarios 2-5. The current fix is enough to unblock scenario 1.

**Update 2026-05-13 (commit 2344fde):** The per-substep refactor landed. `run_binding_yielding`, `run_xtal_workflow_with_vars_yielding`, `run_build_pipeline_yielding`, `run_role_pipeline_yielding`, and `execute_prepared_agent_run_yielding` mirror the original `&mut self` methods but acquire/release the kernel mutex around every `x07` CLI subprocess and every agent stream event. `run_autopilot_yielding`'s AutoBuild and AutoRealize arms now call the yielding variants. End-to-end re-test against real toolchain (`scenario-1-text-utils`, fresh workspace):

- 8/8 `curl --max-time 3 /v1/health` probes during the autopilot run returned HTTP 200 in <2ms.
- Session progressed through `intent_ready → trust_review` while the role pipeline (real claude as architect, real codex as coder) drove 6 realize rounds + 5 review rounds + 7 verify passes (118 ops, 21 timeline turns).
- Bundle snapshot: `target/stress-pass/scenario-1-text-utils/` — `current_rung: local_preview`, `posture_color: green`, `current_step: lint`.

The "real codex repeatedly produces a scaffold that fails the spec, real claude reviewer keeps saying revise" loop is the same observed-with-fake pattern. The autopilot's no-progress guard plus the role pipeline's `max_review_rounds` bound the runaway. F3 is now fully resolved at the daemon level; production-blocker status removed.

---

## F4 — Proof-support warnings not surfaced when real x07 can't prove

**Severity:** FIX.

**Observed:** Real `x07 verify --prove` returned `ok: true` but with two `WXTAL_VERIFY_PROVE_UNSUPPORTED` warnings:

```
app.main.run_v1 | X07V_NO_CONTRACTS | target function has no requires/ensures/invariant clauses
toy.sorter.sort_u8_asc | X07V_UNSUPPORTED_HEAP_EFFECT | x07 verify does not support heap/pointer effect "bytes.set_u8" in the certifiable pure subset
```

This is *exactly* the kind of information the Trust Card should surface — "we can't formally prove this; here's why." Today, the TrustCard would just show `87% proof coverage` (canned in the fake) and no warning. With real x07, posture-pending stays "pending" because of F3 hang, but even if it resolved, the warnings wouldn't be visible.

**Fix sketch:** `crates/loom-core/src/trust_posture.rs` should fold the verify-diag's `WXTAL_VERIFY_PROVE_*` warnings into `TrustPosture.assumptions` (or a new `proof_support_notes` field), and TrustCard.svelte should render them inline below the BUDGET/PROOFS grid.

**Status:** **FIXED** by the Tier-1.5b proof-support pass. See the later F4 section for the implemented `proof_support_notes` projection and TrustCard panel.

---

## F5 — x07 discovery picks sibling debug build over `~/.x07/bin/`

**Severity:** NOTE (informational).

**Observed:** Health snapshot shows `command.source = /Users/webik/projects/x07lang/x07/target/debug/x07` — Studio's discovery prefers the sibling repo's debug binary over the toolchain at `~/.x07/bin/x07`. This is good for developers (always uses the latest build) but should be documented; a user with stale debug artifacts could get unexpected behavior.

**Fix sketch:** Document the discovery order in `docs/CYCLE_2_NOTES.md` or `docs/V0_1_STATUS.md`. Optionally make the order configurable via env var.

**Status:** **DOCUMENTED** in `docs/V0_1_STATUS.md` under "x07 toolchain discovery order."

---

## Scenarios 2-5 — not run this pass

Scenario 1 surfaced a production-blocking daemon HTTP hang (F3) that prevents the harness from polling state during a real run. Continuing with scenarios 2-5 in this session would just re-trigger the same hang without producing new information. The remaining scenarios are deferred until F3 is diagnosed and fixed.

The scenario config files (`scripts/scenarios/scenario-2..5-*.json`) are in place. The recording harness (`scripts/stress_pass.py`) works against any responsive daemon. Re-running scenarios 2-5 after F3 is a contained follow-up — should be 1-2 hours of operator time once the daemon is responsive.

## Real-vs-fake delta (so far)

| Surface | Fake | Real |
|---|---|---|
| HealthRow doctor | green "ok" → "ready" | green "ready" |
| HealthRow lockfile | green "verified" | amber "stale" (no lockfile yet) |
| HealthRow migrate | disabled "up to date" | amber "schema → 0.5" |
| HealthRow overall | green | red |
| TrustCard time-to-posture | instant | not observed (UI hung, posture wasn't fetched) |
| Verify diag | canned `ok:true` clean | real `ok:true` + 2 proof-support warnings |
| AGENT.md content | "# Connected E2E workspace" | full 100+ line x07 0.2.10 agent operating guide |
| Spec files | one toy.sorter | toy.sorter + app.main, both real specs |
| Implementation | seeded `view.to_bytes` body | real `view.to_bytes` + `bytes.set_u8` loop body |
| Daemon HTTP responsiveness | always under 50ms | **freezes after ~30s of subprocess activity** |

The real toolchain actually *delivers more* than the fake — proper AGENT.md, proper specs, multiple modules, real verify diag with proof-support notes. The work is solid; the daemon's HTTP layer is the blocker.

---

## F6 — Intent heuristic misses Unicode / text-normalization prompts

**Severity:** FIXED.

**Observed (scenario 5):** the user typed *"Build a normalize-and-casefold text helper that accepts UTF-8 bytes, NFC-normalizes, then casefolds. Reject non-UTF-8 with a structured error."* Real claude clarified, real codex implemented, real x07 verified, but the impl was a 31-line identity passthrough (`view.to_bytes(bytes.view(payload))`). Reviewer kept saying "revise"; autopilot looped 6 realize attempts; phase eventually settled at `trust_review` with scaffold-only=true. UI rendered correctly throughout (HTTP responsive, Process Lane live), but the agents were grinding on nothing useful.

**Root cause:** `intent_packet_from_raw` has a chain of `has_any(...)` keyword tests for known archetypes (sort/greet/calc/parser/validator/...). None of them recognized `normalize`, `casefold`, `unicode`, `utf-8`, `nfc`. The text fell through to the generic default `("app.main", "run_v1")`. The scaffolded spec then had no `requires/ensures` describing normalization — just `bytes -> bytes` — so codex (correctly) couldn't infer the semantic and emitted identity.

**Evidence:**
- Recorded session: `target/stress-pass/scenario-5-collab/workspace/spec/app.main.x07spec.json` — operation has no requires/ensures.
- `src/app/main.x07.json` — body is `["view.to_bytes", ["bytes.view", "payload"]]` (pure identity).
- Op log shows 6× `agent.realize.openai-codex` + 5× `review.round` (all `revise`) in a row.

**Fix (commit on top of `0f360d7`):** extended `intent_packet_from_raw`'s archetype table:
- `is_text_normalize` (normalize / casefold / nfc / nfd / unicode / utf-8 / utf8) → `app.text.normalize_v1`
- `is_checksum` (checksum / crc32 / hash digest / fingerprint) → `app.checksum.digest_v1`
- `is_codec` (cbor / msgpack / json codec / encode-decode) → `app.codec.roundtrip_v1`
- `is_compress` (compress / zstd / gzip / deflate) → `app.compress.roundtrip_v1`

Regression test `formalize_intent_recognizes_text_normalization_intents` exercises three text-normalize phrasings against the heuristic.

**Status:** **FIXED.** The heuristic now picks a meaningful target for Unicode-shaped prompts. Followup F7 lands the spec-enrichment piece — see below.

---

## F7 — Architect role emits a dummy log; scaffolded spec ships to coder with no semantic content

**Severity:** **FIXED** (deterministic floor; agent-driven enrichment for novel intents deferred).

**Observed (scenario 5, post-F6):** even after F6 routed the normalize-and-casefold prompt to `app.text.normalize_v1`, the on-disk spec still came out as `bytes -> bytes` with `doc: ""`, `requires: []`, `ensures: []`. Codex had a meaningful target id but no behaviour description to implement against, so it emitted `view.to_bytes(bytes.view(payload))` — an identity passthrough. The reviewer voted `revise`. Loop stalled at scaffold_only=true.

**Root cause:** the role-pipeline architect stage was a no-op. It appended one cosmetic op-record (`role.stage.confirm_spec` with the literal note *"Spec is already approved; architect lane confirmed the contract boundary."*) and immediately handed off to the coder. The architect's actual job — owning the spec contract — was never wired up.

Even when run from the build pipeline (not the role pipeline), `x07 xtal spec scaffold` only takes `--module-id / --op / --param / --result`. There is no flag to seed the operation `doc` or `requires/ensures`. So a freshly scaffolded spec has no semantic content for downstream agents to grip, regardless of which pipeline produced it.

**Evidence:**
- `target/stress-pass/scenario-5-collab/workspace/spec/app.main.x07spec.json` — `doc: ""`, `requires: []`, `ensures: []`, `params: [{name:"payload", ty:"bytes"}]`.
- `crates/loom-core/src/kernel.rs:1212` and `:2430` (pre-fix) — both role-pipeline call sites passed the same hard-coded "Spec is already approved" string with no spec read.

**Fix:** new module `crates/loom-core/src/architect.rs` carries a deterministic archetype-semantic table mapping `(module_id, entry)` (the same keys F6 routes to) to a concrete behaviour description. The build pipeline now calls `kernel.architect_enrich_after_scaffold(...)` immediately after `spec.scaffold` succeeds: it reads the scaffolded spec JSON, finds the operation whose `name` matches `{module_id}.{entry}`, and fills the empty `doc` field with the archetype description. The merge is conservative — a non-empty existing `doc` is preserved verbatim, and the call is idempotent. The role pipeline's architect-stage log now reads the latest `architect.enrich_spec` op-record and reports what was actually written (`Architect enriched app.text.normalize_v1 spec with archetype contract before handing off to the coder.`) instead of the previous canned string.

The archetype table covers the F6 targets — `app.text.normalize_v1`, `app.checksum.digest_v1`, `app.codec.roundtrip_v1`, `app.compress.roundtrip_v1` — plus `toy.sorter.sort_u8_asc`, `app.greeter.greet_v1`, `app.calculator.compute_v1`, `app.parser.parse_v1`, `app.validator.validate_v1`, `app.cli.run_v1`, `app.service.handle_v1`. Each entry carries a 2-3 sentence behaviour description naming the inputs, the outputs, the failure mode, and any non-obvious invariant (NFC + casefold ordering, deterministic digest, roundtrip equality, stable sort, etc.).

**Why `doc` only (this pass) and not predicates:** `requires`/`ensures` accept structured S-expression predicates that `x07 xtal spec check` validates strictly. A wrong predicate kills the whole flow. The conservative first move is to enrich the freeform `doc` field, which `spec.check` accepts unconditionally and which is what the coder LLM actually reads. Predicate-based enrichment (length bounds, idempotence checks) is a Tier-1.5 follow-up. Agent-driven enrichment for prompts that hit the generic `app.main.run_v1` fallback is a Tier-2 follow-up.

**Tests:**
- `architect::tests` (8 tests) cover lookup hits/misses, doc merging into a JSON spec value, idempotence on disk, and the no-op path for unrecognised archetypes.
- `kernel::tests::architect_enrich_after_scaffold_writes_doc_for_known_archetype` exercises the full intent → vars → enrichment path: a normalize-and-casefold session ends with the on-disk spec carrying a doc string containing "NFC" and "casefold", plus an `architect.enrich_spec` op-record with `doc_added: true` in the session timeline.
- `kernel::tests::architect_enrich_after_scaffold_is_quiet_for_unknown_archetype` confirms the unknown-archetype path still appends a visible op-record (with `archetype_recognized: false`) instead of silently no-op-ing.

**Status:** **FIXED** (Tier-1 deterministic floor + Tier-2 agent enrichment).

### Tier-2 — Architect agent enrichment for novel intents

The Tier-1 floor only fires for intents the F6 heuristic recognised. When the heuristic falls through to the generic `app.main.run_v1` default — i.e. the user typed something the keyword table doesn't catch — the spec's `doc` stays empty and the coder is back to guessing.

Tier-2 closes that gap inside the role pipeline. After `role.pipeline.started` and before `role.stage.confirm_spec`, the yielding pipeline:

1. Resolves the Architect role to a real agent (default: `claude-code`).
2. Reads the scaffolded spec on disk and checks whether the matching operation's `doc` is empty (`crate::architect::operation_doc_is_empty`). If Tier-1 already filled it, skip — no extra subscription minutes spent.
3. Builds a focused architect handoff (`agent_architect_enrich_handoff_from_session`) and spawns `claude -p --output-format stream-json --add-dir <workspace> <prompt>` through the existing F3-safe yielding executor. The agent has zero write roots and a single allowed verb: `intent.architect.enrich`.
4. Parses the architect's `spec_enrichment` event line out of stdout (kind allowlisted in `parse_structured_agent_event`), validates the schema marker, and merges the `doc` field into the spec via `architect::apply_agent_enrichment_to_spec` — same conservatism as Tier-1, never overwrites existing `doc` content.
5. Appends an `architect.enrich_spec` op-record naming the agent. The role pipeline's `confirm_spec` log then reflects the agent-driven enrichment (`Architect agent \`claude-code\` drafted a behaviour contract for app.main.run_v1 …`) instead of the canned dummy string.

Failure modes are silent: if claude isn't installed, prep returns an error and the pipeline appends a `confirm_spec` log explaining and proceeds; if the subprocess crashes, same; if it emits no usable event, the op-record records `doc_added: false, agent_id: "claude-code"` so the timeline shows the agent ran even though nothing changed.

Output protocol (kept narrow on purpose):

```json
{"schema_version":"x07.studio.agent_event@0.1.0","kind":"spec_enrichment","doc":"…","examples":["…"]}
```

Only `doc` is currently merged into the spec; `examples` are stored on the op-record for a future cycle that pipes them into `IntentPacket.examples`. Predicates (`requires`/`ensures`) are still off-limits to both tiers — the strict `spec.check` gate makes them risky to synthesize without a verification harness.

**Cost gate:** Tier-2 fires at most once per session per role-pipeline turn, only when (a) an architect agent is configured and command-available, and (b) the spec's operation `doc` is still empty after Tier-1. Subscription-only contract preserved (`build_realize_subscription_command` reused — never `--bare`/`--oss`).

### Tier-2 — real-toolchain validation pass

Driven `2026-05-13`, fresh workspace `target/stress-pass/scenario-tier2/workspace`, daemon on `127.0.0.1:7760`, real `claude 2.1.140` + `codex 0.130.0` + `x07 0.2.10`. Intent (chosen to fall through every F6 archetype keyword): *"Build a deduplicator that takes a stream of u8 bytes and returns only the first occurrence of each value, preserving original order. Reject empty input with a structured error."*

**First run surfaced two real production bugs that the unit tests missed:**

**F8 — `claude -p` variadic flags swallow the trailing prompt positional.** The pre-existing `build_realize_subscription_command` arrangement was `claude -p ... --add-dir <workspace> <prompt>`. claude's commander.js parser treats `--add-dir <directories...>` as variadic and consumes every following token until the next flag, so the prompt got eaten and claude errored `"Input must be provided either through stdin or as a prompt argument when using --print"`. The architect-enrich subprocess produced 0 chars of stdout and timed out at the budget ceiling on every attempt. Fix: reorder the args so `--add-dir <workspace> ... --allowedTools "..."` lands well before the prompt, then prepend a literal `--` token to force commander out of option parsing. New regression assertion in `synthesis::tests::build_realize_command_uses_subscription_flags_for_claude` checks `args[len-2..] == ["--", prompt]`.

**F9 — claude `--output-format stream-json` wraps the agent's text inside `{"type":"assistant","message":{"content":[{"type":"text","text":"..."}]}}` envelopes, often with surrounding markdown code fences.** Our `parse_structured_agent_event` only matched top-level `{"schema_version":"x07.studio.agent_event@0.1.0",...}` lines, so even a perfectly-formed `spec_enrichment` event was invisible when claude wrapped it. Codex's `codex exec --json` happens to emit those at the top level (line-oriented), so the bug only manifested for claude-as-agent. Fix: when top-level parse fails, try `parse_structured_agent_event_from_claude_wrapper` which extracts the `content[].text` (or top-level `result` for `{"type":"result",...}`), strips markdown code fences (handles ` ```json ` / ` ``` ` prefixes), scans for the schema marker, and walks brace depth to extract the JSON object. Four new regression tests cover: claude assistant envelope, result envelope, prose-prefixed JSON, no-marker = `None`.

**Third bug — Tier-2 budget was floored at the 8-second confirm-spec stage budget.** Pre-Tier-2 the architect stage was a no-op log; 8s was generous. Tier-2's 90s default got clamped DOWN to the stage budget. Fix: floor at 90s in the role-pipeline budget derivation, and raise `roles::default_routing`'s architect-stage `wall_clock_ms` from `8_000` to `90_000` so the budget is honest.

**Second run, post-fixes:**

- 169 ops in the timeline, phase reached `trust_review` cleanly.
- `agent.architect_enrich.claude-code` finished `succeeded`.
- `agent.event.claude-code.spec_enrichment` op-record was emitted with the parsed structured payload.
- `architect.enrich_spec` op-record carries `doc_added: true, agent_id: "claude-code"`.
- `role.stage.confirm_spec` reads: *"Architect agent `claude-code` drafted a behaviour contract for `app.main.run_v1` before handing off to the coder."*

Resulting spec on disk (`target/stress-pass/scenario-tier2/evidence/app.main.x07spec.json`, `operations[0].doc`):

> *"Given a non-empty sequence of u8 values as input, return a single collected list containing each distinct value in the order it first appeared in the input; subsequent occurrences of a value already seen are dropped. If the input is empty, return a structured error with the tag EmptyInput instead of an empty list. The operation imposes no maximum input length and must preserve first-occurrence order exactly."*

This is what F7-Tier-2 was supposed to do: read a novel-intent prompt the heuristic doesn't recognise, ask the architect agent to draft a real behaviour contract, write it into the scaffolded spec, hand off to the coder. End-to-end on real subscription CLIs.

**HTTP stayed responsive throughout** — 5/5 health probes returned 200 in <10ms during the architect-enrich subprocess. F3 fix still holds.

**Evidence bundle:** `target/stress-pass/scenario-tier2/evidence/`:
- `session-final.json` — full session snapshot (244 KB, 169 ops).
- `app.main.x07spec.json` — enriched spec on disk.

### Tier-1.5 — archetype `ensures` predicates (deferred; doc-only ships)

The hypothesis was simple: extend the `ArchetypeSemantic` table with structured `ensures` predicates that `x07 xtal spec check` accepts (length preservation for sort, length upper bound for normalize, non-empty for greeter). The predicates parse as valid x07 SMT-shape and round-trip through `x07 xtal spec check --project x07.json --input <spec>` cleanly in isolation. So the *spec*-side is fine.

Then a real-toolchain validation pass surfaced two integration bugs (F10, F11) and a deep architectural mismatch (F12):

**F10 — enrichment was only wired into the fresh-scaffold branch.** The xtal-pure template auto-installs `spec/toy.sorter.x07spec.json` (with rich doc + ensures from the canonical x07 example). `should_scaffold_spec` returns `false`, the `else` branch runs `existing_spec_op`, and `architect_enrich_after_scaffold` was never called. Fix: call it in both branches; the merge is conservative so a fully-populated template spec stays untouched.

**F11 — `serde_json::to_string_pretty` fails x07's canonical-JSON gate.** After enrichment, `xtal.verify` rejected the spec with `WXTAL_SPEC_NONCANONICAL_JSON: spec JSON is not in canonical form (run x07 xtal spec fmt --write)`. Fix: when enrichment mutates the spec, follow up with the existing `spec.format` binding (`x07 xtal spec fmt --write --inject-ids`) to re-canonicalize.

**F12 — the impl stub violates every non-trivial predicate.** `impl.sync.write` scaffolds the impl as `["bytes.empty"]` (returns empty bytes). `xtal.verify` runs the SMT prover against this stub. Any predicate that constrains the output (`len > 0`, `len = len(payload)`, etc.) fails because the stub returns zero-length output. The counterexample is *correct* — the stub doesn't satisfy the contract. But this means Tier-1.5 predicates would block the build pipeline at the verify step on every archetype that doesn't have a real impl in the template.

The only template that does ship a real impl is `toy.sorter` (from the xtal-pure template), and its `len_preserved` predicate verifies against the real sort impl. Our archetype path adds predicates to the spec but leaves the impl as the no-op stub — so verify fails.

**Decision:** disable the predicate-merge in `merge_semantic_into_spec` behind a const flag `MERGE_ENSURES_IN_BUILD = false`. The archetype table keeps the predicates declared (so a future "Tier-1.5b" pass can run them after the coder writes a real impl). Doc-only enrichment ships now. Validation confirms: the greeter scenario now reaches `trust_review` cleanly with `architect.enrich_spec [doc=True, ensures=0]`, two `xtal.verify -> succeeded` calls, and a `summary.plain_english` op.

**Where Tier-1.5 leaves us:**

| Layer | Status |
|---|---|
| Archetype-table schema with `ensures` field | ✅ shipped |
| 3 declared predicates (sort len-preserved, greeter non-empty, text-normalize doc-only) | ✅ ship in source |
| `merge_ensures_into_spec` helper (preserves user content) | ✅ shipped (called by tests, gated off in build) |
| Build-pipeline merge invocation | ❌ gated off, see F12 |
| spec.format canonicalization after enrichment | ✅ shipped (F11 fix) |
| Existing-spec branch enrichment | ✅ shipped (F10 fix) |

**Path forward (Cycle-8 or later):** add an "architect contracts" stage that runs AFTER the coder produces real impl, before `xtal.verify`. That stage merges the archetype `ensures` from the table (already declared!) and validates against the real impl. Then flip `MERGE_ENSURES_IN_BUILD` and the existing infrastructure carries the rest.

**Evidence bundle:** `target/stress-pass/scenario-tier15/evidence/`:
- `session-greeter-final.json` — full session snapshot (trust_review reached).
- `app.greeter.x07spec.json` — doc-only enriched spec.
- `xtal.verify.diag.json` — verify ok (proof-support warnings only, no counterexamples).

### Tier-1.5b — predicate promotion after real impl (FIXED)

The plan worked. Tier-1.5b lands a new sequence in the build pipeline:

1. `synthesis.template` (or coder agent) writes a real impl.
2. `architect_promote_predicates_after_impl`:
   - Reads the archetype's `ensures` predicates.
   - Merges them into the spec (only when ensures is empty).
   - **Mirrors the same clauses into the impl file** — preserving the body, just adding the contract metadata to the `defn` decl.
   - Runs `spec.format` to canonicalize.
   - Appends an `architect.enrich_spec` op-record with `ensures_added` count.
3. `impl.check` and `xtal.verify` re-run with the real impl + the predicates. The prover proves the contract against the real code.

The mirror step is critical: without it, `x07 xtal impl check` raises `EXTAL_IMPL_CONTRACT_MISSING` because the spec and impl `ensures` arrays don't match.

**Real-toolchain validation (greeter scenario):**

```
[trust_review] architect.enrich_spec  [doc=True,  ensures=0]   ← Tier-1
[trust_review] spec.check             succeeded
[trust_review] impl.sync.write        (stub)
[trust_review] impl.check + xtal.verify  succeeded             ← stub passes (no contract)
[trust_review] synthesis.template     (real greeter impl)
[trust_review] architect.enrich_spec  [doc=False, ensures=1]   ← Tier-1.5b fired
[trust_review] spec.format            succeeded
[trust_review] impl.check             succeeded                 ← mirror fix worked
[trust_review] xtal.verify            succeeded                 ← real impl + predicate PROVED
[trust_review] summary.plain_english  succeeded
[trust_review] autopilot.decision.complete
phase: trust_review
```

Both files on disk carry the same predicate:

**Spec** (`spec/app.greeter.x07spec.json`):
```json
"ensures": [{"id": "result_nonempty", "expr": [">", ["bytes.len", "__result"], 0]}]
```

**Impl** (`src/app/greeter.x07.json` — body preserved, ensures mirrored):
```json
"defn": {
  "name": "app.greeter.greet_v1",
  "body": ["begin", ["let", "n", ["bytes.len", "payload"]], ...],
  "ensures": [{"id": "result_nonempty", "expr": [">", ["bytes.len", "__result"], 0]}]
}
```

**Evidence bundle:** `target/stress-pass/scenario-tier15b/evidence/`:
- `session-final.json` — autopilot.decision.complete reached.
- `app.greeter.x07spec.json` — spec with promoted predicate.
- `greeter.x07.json` — impl with mirrored predicate + real body.
- `xtal.verify.diag.json` — `ok: true`, predicate proved.

### Examples pipe — Tier-2 agent examples → IntentPacket

Tier-2's `spec_enrichment` event carries an `examples[]` array alongside `doc`. Previously dropped on the floor. `ingest_architect_enrichment` now merges those into `IntentPacket.examples` (deduped against existing entries; capped at 5 per round to bound a pathological agent output). The realize prompt and the UI's plain-English summary now both pick up the architect's worked input → output illustrations.

Test coverage: `ingest_architect_enrichment_writes_doc_from_recorded_event` extended to assert the example landed on the intent.

### F4 — proof-support warnings in TrustCard (FIXED)

Real `x07 verify --prove` emits diagnostic codes that explain *why* the prover left a target unverified:

- `X07V_NO_CONTRACTS` — target function has no requires/ensures/invariant clauses.
- `X07V_UNSUPPORTED_HEAP_EFFECT` — x07 verify does not support heap/pointer effect "bytes.set_u8" in the certifiable pure subset.
- `WXTAL_VERIFY_PROVE_SUPPORT` / `WXTAL_VERIFY_PROVE_UNSUPPORTED` — proof-attempt summary lines.
- `EXTAL_VERIFY_PROVE_COUNTEREXAMPLE` — solver found an input that violates the predicate.

Previously these landed in `target/xtal/xtal.verify.diag.json` and were ignored by the UI. The TrustCard rendered a `Math.round(proved_pct)%` figure with zero context.

Now: `trust_posture::current` reads the diag, filters for proof-support codes, and populates a new `proof_support_notes: Vec<ProofSupportNote>` field on `TrustPosture`. The TypeScript type + the SvelteKit TrustCard component were updated in lockstep. A new collapsible "Proof support" panel renders below the BUDGET/PROOFS grid with severity-colored borders (red for error, amber for warning).

Test counts now: 137 loom-core tests (was 131 before Tier-1.5b; +6 for predicate promotion, impl mirror, existing ingest test extended, plus the two new codec/compress predicate assertions). Web: 70/70 (TrustCard new field optional + backward-compatible).

### Predicate set expansion + visual validation pass

After Tier-1.5b landed, I expanded the predicate-bearing archetype set:
- `app.codec.roundtrip_v1` — `len(__result) = len(payload)` (roundtrip = identity, semantically + matches the identity synthesis floor).
- `app.compress.roundtrip_v1` — same shape, same justification.

Sandbox-validated each predicate end-to-end with real `x07 xtal spec check --project x07.json --input <spec>`: both return `ok: true`, zero diagnostics. At that point the archetype table shipped 4 provable predicates (sort `length_preserved`, greeter `result_nonempty`, codec `length_preserved`, compress `length_preserved`).

**C2 update 2026-05-13:** parser, validator, and calculator now carry post-implementation predicates too:
- `app.parser.parse_v1` — `len(__result) = len(payload)` against an explicit byte-copy template.
- `app.validator.validate_v1` — `len(__result) = 1` against a one-byte status template.
- `app.calculator.compute_v1` — `len(__result) <= 16` against a bounded copy template.

Real Studio build-path validation used isolated daemon workspaces:
- Parser: `target/stress-pass-c2-parser/workspace`, session `172fada6-2286-41fb-b658-d6aa7a51cefc`, `ensures_added: [0, 1]`, verify statuses `succeeded, succeeded`.
- Validator: `target/stress-pass-c2-validator/workspace`, session `fcb826f1-2031-460e-b8b9-d79ff1c983e6`, `ensures_added: [0, 1]`, verify statuses `succeeded, succeeded`.
- Calculator: `target/stress-pass-c2-calculator/workspace`, session `bd6109cc-0164-40a1-a9c4-168956bca86a`, `ensures_added: [0, 1]`, verify statuses `succeeded, succeeded`.

**Visual validation** of F4 done with real toolchain: a fresh sort autopilot session through the SvelteKit UI confirmed the TrustCard renders the new "PROOF SUPPORT 2 notes" collapsible panel with the two real x07 warnings emitted from `x07 verify --prove`:

1. `WXTAL_VERIFY_PROVE_SUPPORT` — *"Proof support summary (first diagnostic per entry): entry | code | message toy.sorter.sort_u8_asc | X07V_UNSUPPORTED_HEAP_EFFECT | x07 verify does not support heap/pointer effect 'bytes.set_u8' in the certifiable pure subset"*
2. `WXTAL_VERIFY_PROVE_UNSUPPORTED` — *"Proof attempt is unsupported for 'toy.sorter.sort_u8_asc' (world='solve-pure', policy='balanced'). See report: target/xtal/verify/prove/toy/sorter/sort_u8_asc.report.json."*

Both render in the new panel with amber severity borders (warning class), each with code + target + message. Toggle interaction works (collapse/expand). Posture color stays `local_preview` green (verify passed despite the support warnings, which is correct — the warnings are informational, not blockers).

**Evidence bundle:** `target/stress-pass/scenario-visual/evidence/`:
- `full-page.png` — viewport with TrustCard + timeline.
- `trust-card-zoom.png` — TrustCard sidebar only.
- `trust-card-with-proof-notes.png` — full-page with proof-support panel expanded.
- `xtal.verify.diag.json` — the source diagnostic the posture is reading from.

**Tests:** loom-core grew to 125 passing (was 109 after Tier-1, 121 after Tier-2 wiring, +4 after F8/F9 fixes). New coverage:
- `architect::tests::operation_doc_is_empty_*` (3) — checks the gate predicate.
- `architect::tests::agent_enrichment_*` (4) — event-parser validation: schema marker, kind tag, non-empty doc, examples array.
- `architect::tests::apply_agent_enrichment_*` (2) — disk merge, idempotence, user-doc preservation.
- `kernel::tests::parse_structured_agent_event_accepts_spec_enrichment_kind` — confirms the agent_event allowlist accepts the new kind.
- `kernel::tests::ingest_architect_enrichment_writes_doc_from_recorded_event` — full ingest: pre-populates a fake `agent.event.<agent>.spec_enrichment` op-record, calls ingest, asserts spec on disk + `architect.enrich_spec` op-record + agent-named stage note.
- `kernel::tests::ingest_architect_enrichment_records_visible_op_when_no_event_emitted` — silent-agent path still appends a visible op-record so the timeline shows the run happened.

---

## F13 — scenario 2 CSV repair loop does not fire

**Severity:** FIX — blocks public-beta A5 completion.

**Observed:** A direct real-daemon probe of `scenario-2-csv-repair` on 2026-05-13 reached `trust_review` with 28 ops and no `xtal.repair` operation. The scenario's acceptance requires the strict CSV quote constraint to make `xtal.verify` fail, then trigger the repair loop. Instead, `xtal.verify` succeeded with proof-support warnings only.

**Evidence:**

- Session: `fd5eb1be-2ae6-45f8-867e-9797d359154c`
- Bundle: `target/stress-pass/scenario-2-csv-repair/`
- Real toolchain: `x07 0.2.10`, `claude 2.1.140`, `codex 0.130.0`
- `target/stress-pass/scenario-2-csv-repair/workspace/spec/app.parser.x07spec.json` has `ensures: []` and `ensures_props: []`.
- `target/stress-pass/scenario-2-csv-repair/workspace/target/xtal/verify/summary.json` reports `coverage_outcome: "pass"` and `prove_outcome: "warn"`, not a failing strict-quote diagnostic.
- `target/stress-pass/scenario-2-csv-repair/op-log.json` has no `xtal.repair` op.

**Root cause:** `app.parser.parse_v1` remains doc-only in the archetype table. The deterministic parser synthesis floor writes a length-prefixed echo implementation, and the post-implementation predicate promotion path has no parser predicate to promote. With no formal contract for malformed quote rejection, `xtal.verify` has nothing strict to fail against, so the repair loop never becomes eligible.

**Original fix direction:** Complete the C2 parser predicate expansion, or add a scenario-specific pre-seeded failing contract/workspace setup. The shipped fix below uses the scenario-specific fixture route and makes a real `xtal.verify` failure reachable.

**Status:** **FIXED** on 2026-05-13.

Fix landed in two parts:
- C2 predicate expansion added parser / validator / calculator post-implementation predicates and matching deterministic templates.
- `scenario-2-csv-repair` now declares `workspace_fixture: scripts/scenarios/fixtures/scenario-2-csv-repair`; `stress_pass.py init` copies that fixture into the evidence bundle workspace. The fixture contains strict CSV examples, including empty input and malformed quote reporting.

Real-daemon validation:
- Bundle: `target/stress-pass-f13-check/scenario-2-csv-repair/`
- Session: `394a5716-3251-4101-a299-cdf4616f8c05`
- Snapshot: `op_count: 23`, `turn_count: 9`, `current_step: repair`
- `target/xtal/verify/summary.json`: `outcome: "fail"`, top code `EXTAL_VERIFY_TESTS_FAILED`, tests `passed: 1`, `failed: 2`.
- Op log: real `xtal.repair` op fired with status `failed`.
- Repair report: `EXTAL_REPAIR_NO_ACTIONABLE_FAILURE` from real `x07 xtal repair --write --semantic-only`.
- Autopilot final decision: `stage: "realize_stalled"`, `action: "user"`, reason "Verification failed and automatic repair did not reach verified evidence; pausing for human review."

This accepts scenario 2 as the intended repair-loop coverage: the strict CSV fixture now reaches real failed verification, real repair, and a clean human pause. It does not claim a production CSV parser implementation.

---

## F14 — scenario 3 os-time trust widening stayed solve-pure

**Severity:** FIX — blocks public-beta A5 completion.

**Observed:** The os-time scenario originally generated a solve-pure starter and stale scenario text expected a synthetic `os-time` world. That did not exercise the real trust widening path.

**Root cause:** Timer intents were not routed to a timer archetype with an OS-time import, and trust posture/ladder logic needed to model real x07 OS execution as `run-os` plus an `os-time` capability rather than a fake world label.

**Status:** **FIXED** on 2026-05-13.

Fixes:
- Timer intents route to `app.timer.elapsed_v1`.
- The timer template imports the OS-time capability and runs through the real `run-os` world.
- Trust posture records worlds `["run-os", "solve-pure"]`, capability `os-time`, amber posture, and world/capability widening deltas.
- Ladder local-preview gate loses `solve-world default` when the implementation widens to `run-os`.
- Scenario docs now assert `run-os` world + `os-time` capability, not a synthetic `os-time` world.

Evidence:
- Bundle: `target/stress-pass-s3-accepted/scenario-3-os-time/`
- Session: `fbd8fac3-f35b-42f7-a3a7-9daec3b9581e`
- Snapshot: `op_count: 29`, `turn_count: 10`, `posture_color: "amber"`.
- Posture: worlds `["run-os", "solve-pure"]`, capability `os-time`, proof support `100%`, proved `87%`.
- Ladder: local preview missing `solve-world default`.

---

## F15 — scenario 4 PBT path used starter manifests and lost repros

**Severity:** FIX — blocks public-beta A5 completion.

**Observed:** The PBT scenario initially ran starter/generated manifests that did not represent the checksum property under test. Early PBT failures also failed to preserve a stable repro path, so the regression endpoint could not reliably lock the counterexample.

**Root cause:** Studio had a PBT button but no scenario-specific manifest generation for the active target, and the regression endpoint did not pass the new `x07 fix --from-pbt` arguments needed to copy the repro and update the regression manifest.

**Status:** **FIXED** on 2026-05-13.

Fixes:
- `pbt.run` writes and runs a real `tests/regress/tests.json` manifest for the Studio target.
- Counterexamples include stable repro IDs and `.x07/cache/pbt/repros/...json` paths.
- `pbt.regression_from` calls `x07 fix --from-pbt <repro> --write --tests-manifest tests/regress/tests.json --out-dir repro/pbt`.
- The locked regression writes `tests/regress/repro/pbt/cca888f40c3ca.x07.json` and `cca888f40c3ca.repro.json`.

Evidence:
- Bundle: `target/stress-pass-s4-accepted/scenario-4-pbt/`
- Session: `4639a1b6-6e61-4345-8fe4-9ed398d40651`
- Counterexample: `studio-pbt-app.checksum-digest_v1-prop_digest_v1_shape`, empty `payload`, failure code `1099`.
- Regression endpoint: `pbt.regression_from` succeeded and appended the locked repro to `tests/regress/tests.json`.

---

## F16 — role-overrides skipped summary evidence and prevented Scenario 5 realize

**Severity:** FIX — blocks public-beta A5 completion.

**Observed:** In a fresh Scenario 5 run with Architect/Coder/Reviewer overrides, the build reached `trust_review` with `src/app/text.x07.json` still equal to the `bytes.empty` stub. Autopilot then stopped at `complete` instead of entering the role pipeline.

**Root cause:** Build auto-template synthesis was correctly skipped when role overrides existed, but the same branch also contained `summary.plain_english` and lint emission. Without a latest summary carrying `scaffold_only: true`, `autopilot::decide_next` had no signal to run `AutoRealize`.

**Status:** **FIXED** on 2026-05-13.

Fix:
- Build still skips local template synthesis when role overrides are present.
- Build now always emits summary/lint evidence on success, so role-overridden sessions surface `scaffold_only: true` and autopilot enters `role.pipeline.started`.
- Regression coverage: `scan_stub_modules_flags_bytes_empty_body` confirms `["bytes.empty"]` bodies are classified as stubs.

Evidence:
- Fresh bundle: `target/stress-pass-s5-summary-fix/scenario-5-collab/`
- Session: `6333cea2-b8ab-40da-8835-6ebb67ddd8e4`
- First verified turn: `summary.plain_english` with `scaffold_only: true`.
- Next autopilot decision: `realize`, followed by `role.pipeline.started`.

---

## F17 — Scenario 5 needed a real Unicode implementation floor

**Severity:** FIX — blocks public-beta A5 completion.

**Observed:** The normalize-and-casefold prompt previously stalled at identity or empty-byte implementations. The spec docs described Unicode behavior, but neither the build nor the fallback had a dependency-backed implementation to replace the scaffold.

**Root cause:** Text targets had no template dependency materialization path. Studio could synthesize local x07AST bodies, but could not add `ext-unicode-rs` to `x07.json`, `x07.lock.json`, and `.x07/deps/` as one atomic implementation step.

**Status:** **FIXED** on 2026-05-13.

Fix:
- Template synthesis can now carry dependencies and materialize them into the workspace.
- Text targets import `ext.unicode`, `ext.unicode.normalize`, and `ext.unicode.casefold`.
- The implementation uses `ext.unicode.normalize.nfkc_basic_v1`, checks `unicode_is_err`, returns `ERR_UTF8` on invalid input, then casefolds with `casefold_basic_v1`.
- Direct validation passes: `x07 xtal impl check --project x07.json` and `x07 xtal verify --project x07.json`.

Important limitation: the released `ext-unicode-rs@0.1.5` package exposes an NFKC-basic normalization floor, not exact NFC. The Scenario 5 evidence should be read as a real Unicode normalize/casefold floor, not a full Unicode NFC conformance claim.

Evidence:
- Workspace: `target/stress-pass-s5-summary-fix/scenario-5-collab/workspace`
- Dependency: `ext-unicode-rs@0.1.5` copied to `.x07/deps/ext-unicode-rs/0.1.5`.
- Impl digest after fallback: `src/app/text.x07.json`, sha256 `38dc250874ca21ced8e6334cb4bbf724ef46e84d18c85fcf6d646439af31153f`.

---

## F18 — streaming agent timeout needed bounded cleanup and preserved output

**Severity:** FIX — prevents stuck role-pipeline subprocesses.

**Observed:** Scenario 5's real Codex invocation can exceed the 60s coder budget while streaming JSON output. The pipeline must preserve partial output, close the subprocess, and continue to the deterministic fallback instead of leaving a zombie process or losing evidence.

**Root cause:** The streaming command runner did not carry timeout handling through the read-loop in a way that explicitly killed the child, aborted reader tasks, and retained partial stdout/stderr.

**Status:** **FIXED** on 2026-05-13.

Fix:
- `run_with_timeout_streaming` now passes the timeout into `wait_with_streaming_output`.
- On timeout, the runner calls `start_kill()`, waits for child exit, aborts reader tasks, keeps partial stdout/stderr, and appends `command timed out after N seconds`.
- Regression coverage: `runner_times_out_streaming_command_with_partial_output`.

Scenario 5 accepted with this bounded behavior: real Codex was invoked and timed out, deterministic template fallback produced the implementation, real x07 verify passed, and real Claude reviewer emitted `verdict: "accept"`.

---

## What scenario 5 also confirmed (positive findings)

- **HTTP stays fully responsive under real subprocess load.** 18 probes during a 3-minute autopilot run all returned HTTP 200 in <2ms. F3 fix holds across multiple realize iterations, multiple reviewer rounds, multiple codex spawns.
- **Real x07 0.2.10 toolchain integration works end-to-end.** AGENT.md is a real 30-line operating guide. `x07.json`, `x07.lock.json`, `x07-toolchain.toml` all populated correctly. Spec files (`toy.sorter`, `app.main`) materialize with proper JSON schema. Tests report + xtal verify diag both write `ok:true`.
- **Role pipeline genuinely invokes the configured agents.** The accepted bundle shows real `agent.clarify.claude-code`, real `agent.realize.openai-codex`, real `agent.review.claude-code`, observed `agent.event.claude-code.review`, and `review.round` with verdict `accept`.
- **The autopilot's scaffold gate fires correctly.** A role-overridden build emits `summary.plain_english` with `scaffold_only: true`; autopilot enters realize, then records a final verified summary with `scaffold_only: false`.
