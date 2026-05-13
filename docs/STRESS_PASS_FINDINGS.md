# Stress-pass findings (Cycle 7)

Live notes from running the real x07 0.2.10 + claude 2.1.140 + codex 0.130.0 toolchain through Studio. Each entry: severity / observation / evidence / fix or open status.

Severity legend:
- **BLOCK** — must fix before production.
- **FIX** — should fix before public beta.
- **NOTE** — visual/UX nit; document.

## Index

| # | Severity | Title | Scenario |
|---|---|---|---|
| F1 | NOTE | HealthRow MIGRATE label reads "schema → 0.5" when `from_schema` is null | 1 |
| F2 | NOTE | TrustCard pending stays "pending" for >1 min on a fresh real workspace | 1 |
| F3 | **FIXED** | Daemon HTTP server returns "Empty reply" after autopilot kicks off real claude/codex (resolved by yielding-autopilot refactor; lock released around AutoClarify subprocess select-loop) | 1 |
| F4 | FIX | Real `x07 verify` produces proof-support warnings the UI doesn't surface clearly | 1 |
| F5 | NOTE | Real x07 picks discovery path from a sibling debug build, not `~/.x07/bin/` | infra |
| F6 | **FIXED** | Intent heuristic doesn't recognize text-normalization / casefold / unicode prompts; falls through to generic `app.main.run_v1` with no semantic guidance, leaving the role-pipeline reviewer in a stall loop | 5 |

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

**Status:** open. Cosmetic; doesn't block scenario flow.

---

## F2 — TrustCard sticks on "pending" while real autopilot is mid-flight

**Severity:** NOTE.

**Observed:** After clicking the recipe and starting autopilot, the TrustCard remains "pending" while real x07 is doing 20-60 seconds of scaffolding/verify work. The Process Lane should show activity in that window, but a posture-pending TrustCard reads as "stuck" to a user who doesn't know the difference.

**Evidence:** Network log shows `/v1/sessions/{id}/trust/posture` returning 404 until the first `posture.captured` op lands.

**Fix sketch:** When session phase is past `intent_drafting` but no posture has been captured yet, the TrustCard should show an animated "Computing trust posture…" state instead of the same idle "pending" copy used at zero-session.

**Status:** open. Would polish well but no functional impact.

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

**Status:** open. Recommended fix before public beta.

---

## F5 — x07 discovery picks sibling debug build over `~/.x07/bin/`

**Severity:** NOTE (informational).

**Observed:** Health snapshot shows `command.source = /Users/webik/projects/x07lang/x07/target/debug/x07` — Studio's discovery prefers the sibling repo's debug binary over the toolchain at `~/.x07/bin/x07`. This is good for developers (always uses the latest build) but should be documented; a user with stale debug artifacts could get unexpected behavior.

**Fix sketch:** Document the discovery order in `docs/CYCLE_2_NOTES.md` or `docs/V0_1_STATUS.md`. Optionally make the order configurable via env var.

**Status:** open. Documentation only.

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

**Status:** **FIXED.** The heuristic now picks a meaningful target for Unicode-shaped prompts. Followup deferred: the spec scaffolder still doesn't extract concrete `requires/ensures` from the intent text — codex still has to guess at the semantic. That's a deeper improvement (probably "ask the architect to draft requires/ensures from the intent before coder runs") and belongs in a future cycle.

---

## What scenario 5 also confirmed (positive findings)

- **HTTP stays fully responsive under real subprocess load.** 18 probes during a 3-minute autopilot run all returned HTTP 200 in <2ms. F3 fix holds across multiple realize iterations, multiple reviewer rounds, multiple codex spawns.
- **Real x07 0.2.10 toolchain integration works end-to-end.** AGENT.md is a real 30-line operating guide. `x07.json`, `x07.lock.json`, `x07-toolchain.toml` all populated correctly. Spec files (`toy.sorter`, `app.main`) materialize with proper JSON schema. Tests report + xtal verify diag both write `ok:true`.
- **Role pipeline genuinely invokes two agents.** Op log shows 6× `agent.realize.openai-codex` (codex doing the implementation) interleaved with `review.round` records that name `claude-code` as the reviewer. The Architect+Coder+Reviewer pipeline is live.
- **The autopilot's stall guard fires correctly.** After enough realize attempts produce scaffold-only summaries, autopilot moves on. The fix from `1ae54f8` continues to hold.
