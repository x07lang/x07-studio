# Loom daemon API

Base path: `/v1`

## Health

- `GET /health`
- `GET /health/snapshot`
- `POST /health/migrate`

Response:

```json
{
  "ok": true,
  "workspace_root": "/path/to/workspace"
}
```

`GET /health/snapshot` runs `x07 doctor`, `x07 pkg lock --project x07.json --check`,
`x07 migrate --check --to 0.5`, and
`x07 project migrate --check --project x07.json`, then projects the result into
`x07.studio.health_snapshot@0.1.0`. `POST /health/migrate` accepts
`{"target":"0.5"}` and creates a `.x07/studio/migrate-backup-*` copy before
write-mode migrations.

## Pointing Vite at a non-default daemon

The Svelte dev server proxies `/v1/**` through `LOOM_DAEMON_ORIGIN`. Example:

```bash
LOOM_DAEMON_ORIGIN=http://127.0.0.1:7729 npm run dev
```

## Workspace Radar

- `GET /workspace/radar`

Response:

```json
{
  "schema_version": "x07.studio.workspace_radar@0.1.0",
  "workspace_root": "/path/to/workspace",
  "xtal_manifest": {
    "path": "arch/xtal/xtal.json",
    "exists": true,
    "modified_unix_ms": 1778500750188
  },
  "spec_count": 1,
  "generated_tests": {
    "path": "gen/xtal/tests.json",
    "exists": true,
    "modified_unix_ms": 1778500750188
  },
  "latest_verify": {
    "path": "target/xtal/verify/summary.json",
    "exists": true,
    "modified_unix_ms": 1778500750188
  },
  "latest_certify": {
    "path": "target/xtal/cert/bundle.json",
    "exists": true,
    "modified_unix_ms": 1778500750188
  },
  "incident_count": 1
}
```

The daemon scans bounded canonical XTAL surfaces from the workspace root:
`arch/xtal/xtal.json`, `spec/**`, `gen/xtal/tests.json`,
`target/xtal/verify/**`, `target/xtal/cert/**`, `target/xtal/violations/**`,
`target/xtal/ingest/**`, and `.x07/studio/**` session state. Browser clients
use this endpoint to render workspace-level readiness instead of inferring all
radar signals from the selected session.

## Bindings

- `GET /bindings`

Returns the canonical rendered binding catalog exposed by `loom-adapters`.

## Sessions

- `GET /sessions`
- `POST /sessions`
- `GET /sessions/{session_id}`
- `GET /sessions/{session_id}/stream` *(SSE; see below)*
- `GET /sessions/{session_id}/turns`
- `POST /sessions/{session_id}/events`
- `POST /sessions/{session_id}/intent/formalize`
- `POST /sessions/{session_id}/intent/revision`
- `POST /sessions/{session_id}/intent/clarify`
- `POST /sessions/{session_id}/intent/answer`
- `POST /sessions/{session_id}/intent/quorum`
- `POST /sessions/{session_id}/intent/image`
- `GET /sessions/{session_id}/agent-contract`
- `POST /sessions/{session_id}/agent-contract`
- `GET /sessions/{session_id}/lint`
- `POST /sessions/{session_id}/lint/{diag_id}/quickfix`
- `POST /sessions/{session_id}/pbt/run`
- `POST /sessions/{session_id}/pbt/regression-from/{repro_id}`
- `GET /sessions/{session_id}/arch-check`
- `POST /sessions/{session_id}/bindings/run`
- `POST /sessions/{session_id}/invoke`
- `GET /sessions/{session_id}/ladder`
- `POST /sessions/{session_id}/ladder/climb`
- `GET /sessions/{session_id}/trust/posture`
- `POST /sessions/{session_id}/diff`
- `GET /sessions/{session_id}/proof/{behavior_id}`
- `GET /sessions/{session_id}/cassettes/ribbon`
- `GET /sessions/{session_id}/incidents/{incident_id}/quickfix`
- `GET /sessions/{session_id}/certificate`
- `POST /sessions/{session_id}/certificate/refresh`
- `GET /sessions/{session_id}/cassette`
- `POST /sessions/{session_id}/cassette/branch`
- `POST /sessions/{session_id}/ask`
- `POST /sessions/{session_id}/incidents/scan`
- `POST /sessions/{session_id}/incidents/{incident_id}/repair`
- `POST /sessions/{session_id}/visual/streampipe/parse`
- `POST /sessions/{session_id}/visual/streampipe/emit`
- `POST /sessions/{session_id}/visual/statemachine/parse`
- `POST /sessions/{session_id}/visual/statemachine/emit`
- `POST /sessions/{session_id}/visual/tasks/parse`
- `POST /sessions/{session_id}/visual/tasks/emit`
- `POST /sessions/{session_id}/artifacts/preview`
- `POST /sessions/{session_id}/docs/preview`
- `POST /sessions/{session_id}/xtal/run`
- `POST /sessions/{session_id}/build`
- `GET /pkg/provides?module=<module-id>`

Create session request:

```json
{
  "title": "Stable sort repair",
  "task_type": "bug_fix"
}
```

Session creation is intentionally only the workspace/session shell. To start
from a user prompt, first `POST /v1/sessions`, then call
`POST /v1/sessions/{session_id}/intent/formalize` with the prompt or existing
spec. The preferred request shape is `{title, task_type}`, but Cycle 4 keeps
the browser-compatible aliases `{intent_text, mode}` at the API boundary.
`task_type` / `mode` defaults to `new_behavior` when omitted.

Dispatch event request:

```json
{
  "event": {
    "event": "formalize_intent",
    "payload": {
      "schema_version": "x07.studio.intent_packet@0.1.0",
      "session_id": "00000000-0000-0000-0000-000000000000",
      "workspace_root": ".",
      "task_type": "bug_fix",
      "targets": [{"module_id": "app.sorter", "entry": "sort_ascending"}],
      "examples": ["[3,1,2] -> [1,2,3]"],
      "constraints": ["reject empty input"],
      "policy_implications": [],
      "ambiguities": [],
      "assumptions": [],
      "witnesses": [{"kind": "desired_behavior", "text": "Keep equal items in order."}],
      "source": {"kind": "text", "raw": "Fix stable sort"}
    }
  }
}
```

Formalize intent request:

```json
{
  "raw": "{\"schema_version\":\"x07.x07spec@0.1.0\",\"module_id\":\"toy.sorter\",\"operations\":[{\"id\":\"op.sort_u8_asc.v1\",\"name\":\"toy.sorter.sort_u8_asc\"}]}",
  "input_mode": "spec",
  "revision_notes": ["Keep the provided spec as the reviewed behavior source."],
  "provider_profile_id": null
}
```

The daemon compiles written plans, voice transcripts, existing `x07.x07spec`
JSON, and incident notes into a `x07.studio.intent_packet@0.1.0`, applies the
legal `formalize_intent` lifecycle transition, and appends a visible
`intent.formalize` operation record with the generated packet in `report_json`.
When `provider_profile_id` names a configured OpenAI-compatible provider,
Studio asks that model for concise intent-polish suggestions and merges only
review metadata such as examples, constraints, ambiguities, assumptions, policy
implications, and witnesses. Provider output is recorded under
`report_json.provider_polish`; deterministic intent generation remains the
fallback when the provider is missing, disabled, unavailable, or returns
unparseable JSON.
For `input_mode: "spec"`, the kernel keeps the provided spec as the auditable
source and derives the target module/entry from `module_id` and the first
operation name or id. Browser clients should use this endpoint instead of
inventing their own connected-mode intent packet.

Agent contract endpoints expose `x07.studio.agent_contract@0.1.0`. `POST`
accepts:

```json
{
  "markdown": "# AGENT.md\n\n## Purpose\n...",
  "prior_hash": "sha256-from-last-read"
}
```

Lint endpoints expose `x07.studio.lint_report@0.1.0` and quickfix records
generated through `x07 fix`. PBT endpoints expose
`x07.studio.pbt_round@0.1.0` and turn counterexamples into regression tests via
`x07 fix --from-pbt`. `arch-check` wraps `x07 arch check` for Shareable and
stricter ladder gates. `/pkg/provides` wraps `x07 pkg provides <module>` for
module discovery in Ask-the-project and ModuleSearch.

Request revision:

```json
{
  "note": "Keep empty input explicit before approval."
}
```

`POST /v1/sessions/{session_id}/intent/revision` records a daemon-owned
`intent.revision.request` operation, stores the note on the session snapshot,
and keeps approval blocked until the intent is repolished through
`intent.formalize`.

Run binding request:

```json
{
  "binding_id": "spec.check",
  "vars": {"input": "spec/app.sorter.x07spec.json"}
}
```

Artifact preview request:

```json
{
  "artifact": "target/xtal/impl-sync.patchset.json"
}
```

Artifact preview response:

```json
{
  "schema_version": "x07.studio.artifact_preview@0.1.0",
  "artifact": "target/xtal/impl-sync.patchset.json",
  "media_kind": "json",
  "bytes_read": 481,
  "truncated": false,
  "text": "{ ... }",
  "json": {
    "schema_version": "x07.patchset@0.1.0",
    "patches": []
  },
  "patchset_preview": {
    "schema_version": "x07.studio.patchset_preview@0.1.0",
    "targets": [
      {
        "path": "src/main.x07.json",
        "note": "Realize approved operation",
        "operations": 2,
        "before_json": {"solve": ["bytes.lit", "todo"]},
        "after_json": {"solve": ["bytes.lit", "ok"]},
        "apply_error": null,
        "truncated": false
      }
    ]
  }
}
```

The daemon only previews paths already recorded in that session's operation
artifacts or report paths, rejects absolute or parent-traversal paths, reads
from the workspace root, and caps preview bodies. Browser clients use this to
turn patchset artifact paths into file-level patch review rows. When the
artifact is an x07 patchset, the daemon also reads each bounded workspace target
and applies the JSON Patch in memory so the browser can show before/after JSON
without writing to disk. Patchset target previews are limited to reviewable x07
project surfaces such as `src/`, `tests/`, `spec/`, `arch/`, `gen/`,
`policy/`, `policies/`, `wit/`, `x07.json`, and `x07.lock.json`. Target-level
read, policy, or patch failures are returned as `apply_error` rows instead of
failing the whole artifact preview.

Documentation preview request:

```json
{
  "doc_ref": "x07/docs/getting-started/agent-quickstart.md"
}
```

Documentation preview response:

```json
{
  "schema_version": "x07.studio.doc_preview@0.1.0",
  "doc_ref": "x07/docs/getting-started/agent-quickstart.md",
  "resolved_path": "/path/to/x07/docs/getting-started/agent-quickstart.md",
  "title": "Agent quickstart",
  "media_kind": "markdown",
  "bytes_read": 8192,
  "truncated": false,
  "snippet": "Use x07 run as the canonical execution front door.",
  "entries": []
}
```

The daemon resolves only `x07/docs/...` refs under the local x07 docs root,
rejects parent traversal, caps file snippets, and returns bounded directory
entries for docs indexes such as `x07/docs/examples`. Browser clients use this
to show the same canonical documentation context that the session doctrine and
agent handoff already name.

Run XTAL workflow request:

```http
POST /v1/sessions/{session_id}/xtal/run
```

Optional request body:

```json
{
  "vars": {
    "proof_policy": "strict",
    "allow_os_world": "false",
    "unwind": "2",
    "max_bytes_len": "12",
    "input_len_bytes": ""
  }
}
```

The daemon derives safe binding variables from the approved intent packet. For
starter workspaces with no `x07.json`, it initializes an `xtal-pure` project,
then records visible operation records for `spec.scaffold`, `spec.check`,
`tests.gen.write`, `impl.sync.write`, `impl.check`, and `xtal.verify`.
For `xtal.verify`, Studio accepts only bounded verification controls from the
browser: `proof_policy=balanced|strict`, `allow_os_world=true|false`, and
positive integer values for `unwind`, `max_bytes_len`, and `input_len_bytes`.
Those values become real `x07 xtal verify` flags in the operation record.

For `incident_repair` sessions, `intent/formalize` persists the user's incident
note as a session-scoped XTAL violation bundle under `.x07/studio/incidents/`.
After approval, `xtal/run` initializes the project if needed, ensures
`arch/xtal/xtal.json` exists for incident resolution, records
`xtal.ingest --normalize-only`, and then records `xtal.improve` against that
bundle. These operations stay in the normal session worklog and artifacts list.

When the approved intent maps to a supported docs example, Studio seeds that
example first and then runs its canonical workflow:

- `workflow.graph`: `docs/examples/agent-gate/xtal/workflow-graph`
- `workflow.lifecycle`: `docs/examples/readiness-checks/x07-sm-arch-contracts-smoke`
- `gateway.core`: `docs/examples/apps/x07-api-gateway`
- `crawl.plan`: `docs/examples/apps/x07crawl`
- `db.guard`: `docs/examples/apps/x07dbguard`
- `atlas.app`: `docs/examples/wasm_showcases/x07_atlas`

Each seeded workflow appends its `project.seed.*`, generation, arch/package,
test, run, bundle, and verification records to the same session worklog. Atlas
also records app profile validation, app trace replay, release pack verification,
provenance, deploy-plan, and SLO evidence. If `X07_VM_VZ_GUEST_BUNDLE` is not
declared, sandbox examples use the explicit OS-backed sandbox bindings with
`--i-accept-weaker-isolation`.

## Providers

- `GET /providers`
- `POST /providers`
- `POST /providers/probe`

Provider probe request:

```json
{
  "profile": {
    "schema_version": "x07.studio.provider_profile@0.1.0",
    "id": "ollama-local",
    "label": "Ollama local",
    "base_url": "http://127.0.0.1:11434/v1",
    "api_key_env": null,
    "api_key": null,
    "api_kind": "openai_compatible",
    "model": "qwen3-coder",
    "default_headers": {},
    "local": true,
    "trust_tier": "local_trusted",
    "probe_mode": "deep",
    "disabled": false
  }
}
```

## Agents

- `GET /agents`
- `POST /agents`
- `POST /sessions/{session_id}/agents/{agent_id}/handoff`
- `POST /sessions/{session_id}/agents/{agent_id}/run`
- `POST /sessions/{session_id}/agents/{agent_id}/approval`
- `POST /sessions/{session_id}/approvals/{op_id}`

Agent profile response:

```json
{
  "schema_version": "x07.studio.agent_profile@0.1.0",
  "id": "openai-codex",
  "label": "OpenAI Codex",
  "command": "codex",
  "args": [],
  "allowed_verbs": ["intent.formalize", "spec.check", "xtal.verify"],
  "mcp_tools": ["x07.search_v1", "x07.context_pack_v1", "x07.exec_v1"],
  "write_roots": ["spec/", "src/", "tests/"],
  "approval_required": true,
  "status": "available",
  "notes": "Remote coding-agent runner gated by x07 session contract."
}
```

Agent handoff response:

```json
{
  "handoff": {
    "schema_version": "x07.studio.agent_handoff@0.1.0",
    "session_id": "00000000-0000-0000-0000-000000000000",
    "agent_id": "openai-codex",
    "prompt_path": ".x07/studio/handoffs/00000000-0000-0000-0000-000000000000-openai-codex.md",
    "command": ["codex", ".x07/studio/handoffs/00000000-0000-0000-0000-000000000000-openai-codex.md"]
  },
  "session": {}
}
```

Agent run request:

```json
{
  "mode": "plan",
  "timeout_seconds": 30
}
```

`mode: "plan"` records a visible supervised launch plan without executing the
agent command. `mode: "execute"` runs the configured agent command from the
workspace root with the handoff prompt path as its final argument. The daemon
first appends a `running` `agent.run.*` operation so clients can poll the session
while the command is active, updates the same operation with streaming
stdout/stderr chunks, then writes the final captured output and succeeded or
failed status. While streaming, the kernel also appends bounded
`agent.event.*` records when output lines report artifact paths, diagnostics,
write activity, or approval/policy requests.

Generated handoff prompts include the approved intent, session contract,
allowed verbs, MCP tools, write roots, required XTAL loop, and an execution
boundary section. That boundary names `x07 run` as the default execution front
door and calls out solve-rr, sandbox/run-os, WASM app, release/provenance, and
SLO/budget lanes when the session evidence implies those gates.
When the approved intent matches a service archetype such as `api-cell`,
`event-consumer`, `scheduled-job`, `policy-service`, or `workflow-service`,
the handoff also embeds local `x07 service genpack schema --archetype ...`
and `x07 service genpack grammar --archetype ...` output so the agent can
draft service-shaped artifacts against the released contract.

Handoff prompts also define a structured agent event JSONL protocol. Agents may
emit one JSON object per line with
`schema_version: "x07.studio.agent_event@0.1.0"` and `kind` set to `artifact`,
`diagnostic`, `write`, or `approval`. The daemon records those as
`agent.event.<agent>.<kind>` operations with any safe artifact path attached,
which gives the browser approval and artifact signals without relying on
free-form terminal text.

The daemon enforces coding-agent readiness at the API boundary. Disabled agent
profiles cannot create handoffs, plans, or runs. `mode: "execute"` also checks
that the configured agent command exists before appending a supervised
`agent.run.*` operation; missing commands must be installed or the profile
command must be updated first. `mode: "plan"` may still record a launch plan
for a non-disabled profile because it does not spawn the command.

During `mode: "execute"`, the launched process receives `X07_STUDIO_*`
environment variables for the session id, agent id, handoff path, allowed
verbs, MCP tools, write roots, approval mode, and event schema. Loom snapshots
bounded workspace source/config files before and after the process. If the
agent changes files outside its write roots plus Studio handoff state, the
final `agent.run.*` operation is marked `failed` even when the process exits
zero, and `report_json.write_audit` contains:

```json
{
  "schema_version": "x07.studio.agent_write_audit@0.1.0",
  "allowed_roots": ["src/", ".x07/studio/"],
  "created": ["src/ok.txt", "private/bad.txt"],
  "modified": [],
  "deleted": [],
  "violations": ["private/bad.txt"],
  "truncated": false
}
```

If the agent profile has `approval_required: true`, `mode: "execute"` first
records a pending `agent.approval.*` checkpoint unless the latest relevant
agent operation is a succeeded approval. A later handoff, plan, or run consumes
that checkpoint and requires a new approval before the next execution. Resolve
the checkpoint with:

```json
{
  "decision": "approve",
  "notes": "Human reviewed the session contract and write roots."
}
```

Intent clarify request:

```json
{
  "agent_id": "claude-code",
  "timeout_seconds": 90
}
```

`POST /v1/sessions/{session_id}/intent/clarify` spawns the supervised
coding-agent runner identified by `agent_id` in a clarify-only mode. The
agent is restricted to the `intent.clarify` verb with no write roots; its
handoff prompt is saved under
`.x07/studio/handoffs/{session}-{agent}-clarify.{md,json}` and asks the
agent to emit 1-3 structured `clarify_question` events or one
`clarify_done` event on the existing
`x07.studio.agent_event@0.1.0` JSONL protocol. The daemon streams
stdout/stderr through the same supervised channel used by `agent.run.*`,
parses the new event kinds, and (on completion) ingests the resulting
questions into a `clarification_history: Vec<ClarificationTurn>` field on
the session's intent packet. Browser clients then render each turn as a
Q&A card directly off `session.intent.clarification_history`.

Intent answer request:

```json
{
  "answers": [
    {
      "question_id": "q1",
      "text": "Reject empty input with an error.",
      "witness_kind": "forbidden_behavior"
    }
  ]
}
```

`POST /v1/sessions/{session_id}/intent/answer` pairs each answer with its
question by `question_id`, fills in the matching `ClarificationTurn`,
appends a typed witness to the intent packet, re-emits the intent through
the reducer (the session stays in `IntentReady`), and records a visible
`intent.clarify.answers` operation.

Build pipeline request:

```http
POST /v1/sessions/{session_id}/build
```

Optional request body:

```json
{
  "vars": { "proof_policy": "balanced" },
  "max_repair_rounds": 3
}
```

`POST /v1/sessions/{session_id}/build` is the Timeline wrapper around
the XTAL workflow. It emits a plain-English `build.stage.start` marker,
runs `run_xtal_workflow_with_vars` (scaffold → spec.check →
tests.gen.write → impl.sync.write → impl.check → xtal.verify), and — on
verification failure — runs up to `max_repair_rounds` rounds of
`xtal.repair --semantic-only --write` followed by another `xtal.verify`.
Stops at `trust_review` (verified) or `human_intervention_required`. On
success, emits `build.stage.done` followed by a deterministic
`summary.plain_english` OpRecord whose `report_json` carries
`x07.studio.plain_english_summary@0.1.0`
(`headline`, `behavior_promises`, `boundaries`, `evidence`,
`run_invocation`, `followups`).

## Timeline and Cycle 2 Operations

`GET /v1/sessions/{session_id}/turns` returns a typed chronological projection
for the browser Timeline. Turn variants include `user_intent`,
`agent_clarify`, `user_answer`, `agent_draft`, `user_approved`,
`build_stage`, `verified`, `incident`, `repair`, `agent_stream`, `mcp_call`,
`quorum_realize`, and `trust_posture_changed`.

Try-It request:

```json
{
  "input_kind": "text",
  "input_text": "[3,1,2]",
  "input_b64": null,
  "input_path": null,
  "argv": [],
  "profile": "sandbox"
}
```

`POST /v1/sessions/{session_id}/invoke` executes the verified artifact through
the x07 CLI using framed stdin for text or bytes input and returns captured
output, stats, proof citations, and the recorded invocation op id.

Shipping ladder:

- `GET /v1/sessions/{session_id}/ladder`
- `POST /v1/sessions/{session_id}/ladder/climb`

```json
{
  "to_rung": "team"
}
```

The ladder projects four rungs: `local_preview`, `shareable`, `team`, and
`production`. Each rung reports satisfied state, missing evidence, artifact
evidence, and explicit `gates` for the browser's shipping review. Climbing
records the trust command associated with the target rung; successful profile
certification also satisfies the matching rung gate even when the profile file
is external to the project.

Cycle 4 trust and review endpoints:

- `GET /v1/sessions/{session_id}/trust/posture` returns
  `x07.studio.trust_posture@0.1.0` with worlds, capability reads, budget
  summary, proof coverage, posture color, and deltas from the latest captured
  posture.
- `POST /v1/sessions/{session_id}/diff` accepts
  `x07.studio.semantic_diff_request@0.1.0` and returns
  `x07.studio.semantic_diff@0.1.0`. Refs can point at `current`, an operation
  id, a timeline turn id, a hash, or a quorum proposal.
- `GET /v1/sessions/{session_id}/proof/{behavior_id}` returns
  `x07.studio.proof_evidence@0.1.0` by joining plain-English behavior promise
  ids with the latest verify/proof artifacts.
- `GET /v1/sessions/{session_id}/incidents/{incident_id}/quickfix` returns
  `x07.studio.quickfix_record@0.1.0` from `.x07-wasm/incidents`,
  `target/xtal/violations`, `target/xtal/ingest`, and the latest repair
  patchset if present.
- `GET /v1/sessions/{session_id}/cassettes/ribbon` returns
  `x07.studio.cassette_ribbon@0.1.0`, an ordered list of replay boundary
  entries under `.x07_rr`.
- `GET /v1/sessions/{session_id}/certificate` returns
  `x07.studio.certificate_summary@0.1.0` from certificate, verify, and trust
  artifacts. `POST /certificate/refresh` runs `xtal.certify` best-effort before
  returning the same summary shape.

Intent quorum request:

```json
{
  "agent_ids": ["openai-codex", "claude-code"],
  "timeout_seconds": 90
}
```

`POST /v1/sessions/{session_id}/intent/quorum` runs the requested agents in
parallel supervised clarify mode, ingests their structured `clarify_question`
events into one shared quorum round, and records a diff summary operation.

Image witness upload uses `multipart/form-data`:

```text
POST /v1/sessions/{session_id}/intent/image
field: file=<image file>
field: mime=image/png
```

The daemon accepts `image/*` uploads up to 8 MiB and stores them under
`.x07/studio/sessions/{session}/images/`.

Cassette endpoints:

- `GET /v1/sessions/{session_id}/cassette`
- `POST /v1/sessions/{session_id}/cassette/branch`

```json
{
  "from_entry": 4,
  "new_title": "Try stricter empty-input behavior"
}
```

Branching creates a sibling session from the source session, replays cassette
entries through the selected index into
`.x07/studio/cassette_branches/{session}/replay`, truncates later session
operations, and records a replay manifest as the branch operation artifact.

Project Q&A request:

```json
{
  "question": "Why is this safe to ship?",
  "agent_id": null
}
```

`POST /v1/sessions/{session_id}/ask` returns a concise answer plus citations
to operation evidence, artifacts, docs, or memory.

Incident endpoints:

- `POST /v1/sessions/{session_id}/incidents/scan`
- `POST /v1/sessions/{session_id}/incidents/{incident_id}/repair`

The scan reads `.x07-wasm/incidents`, `target/xtal/violations`, and
`target/xtal/ingest` and turns incident bundles into visible operations.
Repair records the bounded `xtal.repair` / ingest-improve lane for the chosen
incident.

Visual parse/emit endpoints normalize graph payloads for `streampipe`,
`statemachine`, and `tasks`; the browser visual editor uses the same contract:

```json
{
  "source": {"nodes": [], "edges": []}
}
```

```json
{
  "graph": {"nodes": [], "edges": []}
}
```

## Sync and Memory

- `GET /v1/sync/codes`
- `POST /v1/sync/{code}/claim`
- `GET /v1/memory`
- `POST /v1/memory`

Sync codes point at the daemon's current first session, expire after their
timestamp, and are persisted under `.x07/studio/sync_codes.json` so a restarted
local daemon can still claim unexpired codes. Studio memory is stored locally as
append-only JSONL at `~/.x07-studio/memory.jsonl` unless
`X07_STUDIO_MEMORY_PATH` overrides it.
`POST /v1/memory` accepts a JSON merge-style patch and returns the projected
memory state, which contains preferences, recent projects, and reusable spec
references.

## Session stream (Server-Sent Events)

`GET /v1/sessions/{session_id}/stream` is a `text/event-stream` endpoint
backed by a per-session `tokio::sync::broadcast` hub
(`loom-core::SessionEventBus`). Each frame's `data:` payload is a JSON
object tagged with `kind`:

```text
event: message
data: {"kind":"snapshot","session":{ ... full SessionSnapshot ... }}

event: message
data: {"kind":"op","op":{ ... single OpRecord ... }}
```

`Op` events are emitted on every `AppendOp` / `UpdateOp` dispatch and let
the browser dedupe by `op.id`. `Snapshot` events are emitted on every
other state-machine transition (`FormalizeIntent`, `ApproveSpec`,
`VerificationPassed`, etc.) so phase, room, intent, and contract changes
land in one consistent payload. Axum's `KeepAlive::default()` keeps the
connection warm; idle clients receive a `: keepalive` comment.

## MCP

- `POST /mcp/connect`
- `GET /mcp/{connection_id}/tools`
- `POST /mcp/{connection_id}/call`
- `DELETE /mcp/{connection_id}`

Connect over HTTP:

```json
{
  "endpoint": {
    "transport": "http",
    "label": "x07lang-mcp-http",
    "base_url": "http://127.0.0.1:8314",
    "mcp_path": "/mcp",
    "bearer_env": null,
    "bearer_token": null,
    "default_headers": {}
  },
  "alias": "local-x07-mcp"
}
```

Connect over stdio:

```json
{
  "endpoint": {
    "transport": "stdio",
    "label": "x07lang-mcp-stdio",
    "command": "uvx",
    "args": ["x07lang-mcp"],
    "cwd": null,
    "env": {}
  },
  "alias": "local-x07-mcp-stdio"
}
```

Call tool request:

```json
{
  "name": "x07.search_v1",
  "arguments": {
    "query": "xtal verify"
  }
}
```

## Cycle 3 endpoints

- `POST /v1/sessions/{session_id}/realize/quorum` runs Claude Code and Codex
  realization in staged workspaces and returns `RealizeQuorumRound`.
  `schema_version` defaults to `x07.studio.realize_quorum_request@0.1.0` when
  older clients omit it.
- `POST /v1/sessions/{session_id}/realize/pick` applies the chosen quorum
  proposal, then reruns `impl.check` and `xtal.verify`.
- `POST /v1/sessions/{session_id}/autopilot/start` runs the bounded autopilot
  loop with an optional policy.
- `POST /v1/sessions/{session_id}/autopilot/pause` records a pause decision.
- `GET /v1/sessions/{session_id}/diffs/live` streams normalized `LiveDiff`
  frames extracted from streamed Edit / Write tool-use events.
- `POST /v1/sessions/{session_id}/intent/voice` formalizes a voice transcript
  directly and records the transcript confidence on the intent operation.
- `POST /v1/sync/sessions/{code}/state` persists a session-local state blob on
  a sync code; `POST /v1/sync/{code}/claim` returns it with the session.
- `POST /v1/sessions/{session_id}/ladder/release` submits the selected ladder
  rung through x07lp deploy bindings and records `ReleaseStatus`.
  `schema_version` defaults to `x07.studio.release_request@0.1.0` when older
  clients omit it.
- `GET /v1/sessions/{session_id}/ladder/release/{release_id}` returns the latest
  recorded release status.
- `POST /v1/sessions/{session_id}/incidents/watch` starts a bounded background
  incident watcher for the session and immediately returns the current ingest.
- `POST /v1/sessions/{session_id}/replay/export` writes a local replay capsule.
- `POST /v1/replay/import` imports a replay capsule into the local session
  store.

Streamed agent events continue to arrive on
`GET /v1/sessions/{session_id}/stream` as ordinary `Op` frames with
`agent.event.<agent_id>.stream_<kind>` operation names.
