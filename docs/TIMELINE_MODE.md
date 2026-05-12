# Timeline Mode

Timeline Mode is x07 Studio's default browser surface. It replaces the old
Simple/Expert split with one chronological operating surface that keeps the
whole lifecycle visible:

```text
intent -> clarification -> approval -> build -> verify -> try -> ship -> learn
```

The premise is unchanged: the user-facing artifact is the behavior promise,
not hidden generated code. The difference in Cycle 2 is that every question,
answer, build stage, proof, incident, repair, and follow-up stays in one
auditable timeline instead of moving between separate beginner and expert
screens.

## Core Surfaces

1. **`Composer`** accepts text intent, `/binding <id>` commands, and image
   witnesses. Image upload uses `multipart/form-data` with a `file` field.
2. **`Timeline`** renders typed turns from `GET /v1/sessions/{id}/turns`:
   user intent, agent clarification, user answers, agent drafts, approval,
   build stages, verified summaries, incidents, and repairs.
3. **`ResultPreview`** renders the canonical `summary.plain_english` record,
   including behavior promises, boundaries, evidence, a runnable
   `x07 run ... --stdin` invocation, and follow-up refinements.
4. **`TryItPanel`** runs the verified artifact through
   `POST /v1/sessions/{id}/invoke` and shows output plus proof citations.
5. **`ShippingLadder`** projects the project across `local_preview`,
   `shareable`, `team`, and `production` rungs with missing evidence and a
   controlled climb action.
6. **`NowPanel`** keeps operational controls close to the current work:
   live quorum review, incident scan/repair, cassette history and branching,
   project Q&A, sync-code mint/claim, memory load, and visual graph editing.

## Timeline Contract

Clients no longer infer a user journey by scanning raw operation names. The
daemon projects session state into typed `SessionTurn` variants:

| Turn kind | Source |
| --- | --- |
| `user_intent` | approved or draft intent source |
| `agent_clarify` | `clarify_question` agent events and pending Q&A |
| `user_answer` | `intent.clarify.answers` |
| `agent_draft` | supervised agent draft and artifact evidence |
| `user_approved` | session contract approval |
| `build_stage` | build and XTAL binding operation groups |
| `verified` | `summary.plain_english` with proof-backed evidence |
| `incident` | scanned violation and incident bundles |
| `repair` | `xtal.repair`, `xtal.ingest`, and `xtal.improve` records |

The `?mode=expert` URL parameter is preserved as a compatibility alias that
opens evidence drawers by default. There is no mode toggle and no separate
expert UI state.

## Build And Follow-Up Loop

`POST /v1/sessions/{id}/build` still wraps the canonical XTAL chain:
`spec.scaffold`, `spec.check`, `tests.gen.write`, `impl.sync.write`,
`impl.check`, and `xtal.verify`, with bounded semantic repair on verification
failure. On success it emits `summary.plain_english` with:

- a headline
- behavior promises
- boundaries
- evidence bullets
- a runnable invocation
- deterministic follow-up prompts

Clicking a follow-up sends that refinement back through the same intent path.
The reducer allows refinement from `trust_review`, `certified`, and
`repair_eligible` so users can iterate from proof-backed output instead of
starting a new project.

## Cycle 2 Endpoints

Timeline Mode uses these additions on top of the v0.1 session, binding,
provider, agent, MCP, preview, and build endpoints:

- `GET /v1/sessions/{id}/turns`
- `POST /v1/sessions/{id}/invoke`
- `GET /v1/sessions/{id}/ladder`
- `POST /v1/sessions/{id}/ladder/climb`
- `POST /v1/sessions/{id}/intent/quorum`
- `POST /v1/sessions/{id}/intent/image`
- `GET /v1/sessions/{id}/cassette`
- `POST /v1/sessions/{id}/cassette/branch`
- `POST /v1/sessions/{id}/ask`
- `POST /v1/sessions/{id}/incidents/scan`
- `POST /v1/sessions/{id}/incidents/{incident_id}/repair`
- `GET /v1/sync/codes`
- `POST /v1/sync/{code}/claim`
- `GET /v1/memory`
- `POST /v1/memory`
- `POST /v1/sessions/{id}/visual/{streampipe|statemachine|tasks}/parse`
- `POST /v1/sessions/{id}/visual/{streampipe|statemachine|tasks}/emit`

## Current Limits

- Studio memory is a local JSONL append surface under `~/.x07-studio`; it is
  not a hosted account system.
- Visual editors cover the supported `streampipe`, `statemachine`, and `tasks`
  graph exchange layer; specialized editors for future x07 surfaces should
  build on the same parse/emit contract.
