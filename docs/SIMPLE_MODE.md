# Simple Mode

Simple Mode is x07-studio's default landing for new users. It collapses the
full Seven-Rooms surface into a single guided flow so a non-engineer can go
from `"What do you want to build?"` through a verified, evidence-backed
project without reading any docs or seeing any JSON.

The premise: **the user-facing artifact is the behavior promise, not the
code.** Humans describe intent (voice or text), review what the system
*promises to do*, and approve when aligned. The agent does the engineering.
x07's spec-first architecture makes this natural — specs are the contract,
code is generated, proofs back the promise.

## The five surfaces

1. **`SimpleStart`** — single prompt + microphone + **Begin Building**.
2. **`ClarifyQuestionCard`** — one structured Q&A card per round. The card
   carries the question's `witness_kind` so the answer becomes a typed
   witness on the intent packet rather than a free-form chat turn.
3. **Approve & Build** — one button that fires the orchestrated build
   pipeline (`POST /v1/sessions/{id}/build`).
4. **`SimpleBuildProgress`** — plain-English stage strip
   (*Understanding → Designing → Writing → Testing → Verifying → Done*) plus
   live activity over SSE.
5. **`SimpleResultPreview`** — the `summary.plain_english` record from the
   daemon, surfaced as `headline → behavior promises → boundaries → evidence`.

## When the user sees what

| Phase                  | Stage in Simple Mode           |
| ---------------------- | ------------------------------ |
| no session yet         | `SimpleStart`                  |
| `intent_drafting`      | `SimpleStart` (waiting on user)|
| `intent_ready`         | `ClarifyQuestionCard`s         |
| `spec_draft`/`spec_review`/`spec_approved`/`realization_proposed`/`verify_running`/`repair_eligible` | `SimpleBuildProgress` |
| `trust_review`/`certify_running`/`certified` | `SimpleResultPreview` |
| `human_intervention_required` | `SimpleResultPreview` (needs-help variant) |

## Mode toggle

A header chip lets the user flip to **Expert** at any time. The preference
is persisted in `localStorage` under `x07-studio-mode`. Tests can force a
mode via the `?mode=simple` / `?mode=expert` URL parameter — the toggle
prefers an explicit URL parameter over the persisted value, and the
Playwright suite uses `?mode=expert` to exercise the existing Seven-Rooms
flow without touching shared state.

## Expert ⇄ Simple parity

Nothing is removed in Expert mode. Every Simple action maps to a canonical
binding the Expert surface already exposes:

| Simple action          | Daemon endpoint / binding                  |
| ---------------------- | ------------------------------------------ |
| **Begin Building**     | `POST /sessions` + `POST /intent/formalize`|
| Clarify round          | `POST /intent/clarify` (supervised agent)  |
| Answer a question      | `POST /intent/answer`                      |
| **Approve & Build**    | `draft_spec` + `approve_spec` + `POST /build` |
| Plain-English summary  | `summary.plain_english` OpRecord           |

The build pipeline composes the existing XTAL bindings — `spec.scaffold`,
`spec.check`, `tests.gen.write`, `impl.sync.write`, `impl.check`,
`xtal.verify`, and (on failure) `xtal.repair` with up to three rounds of
semantic-only repair. Stops at verified; certification stays an explicit
Expert action.

## Why structured Q&A cards, not free-form chat

Free-form chat collapses to "just generate code" — exactly the failure mode
XTAL is designed to prevent. Structured cards keep the conversation:

- **auditable** — every question and answer becomes a recordable
  `ClarificationTurn` on the intent packet.
- **typed** — each answer carries a `witness_kind`, so it maps cleanly to a
  spec-level witness rather than a generic comment.
- **bounded** — at most 3 questions per round; max 4 rounds by default.
- **agent-driven** — the coding-agent runner (Claude Code or Codex) emits
  questions via the existing `agent_event` JSONL protocol (new kinds:
  `clarify_question`, `clarify_done`).

## What's intentionally minimal in Cycle 1

- Voice input still uses the browser's Web Speech API; no offline STT model.
- Spec / Realization / Verify / Repair / Trust / Ops rooms are not yet
  decomposed into per-room components — Simple Mode is a parallel surface,
  not a refactor. Future cycles will split `+page.svelte` further.
- The build pipeline stops at `trust_review`. Certification, release, and
  rollout stay in Expert mode.
