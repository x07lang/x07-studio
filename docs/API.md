# Loom daemon API

Base path: `/v1`

## Health

- `GET /health`

Response:

```json
{
  "ok": true,
  "workspace_root": "/path/to/workspace"
}
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
- `POST /sessions/{session_id}/events`
- `POST /sessions/{session_id}/intent/formalize`
- `POST /sessions/{session_id}/bindings/run`
- `POST /sessions/{session_id}/artifacts/preview`
- `POST /sessions/{session_id}/xtal/run`

Create session request:

```json
{
  "title": "Stable sort repair",
  "task_type": "bug_fix"
}
```

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
  "revision_notes": ["Keep the provided spec as the reviewed behavior source."]
}
```

The daemon compiles written plans, voice transcripts, existing `x07.x07spec`
JSON, and incident notes into a `x07.studio.intent_packet@0.1.0`, applies the
legal `formalize_intent` lifecycle transition, and appends a visible
`intent.formalize` operation record with the generated packet in `report_json`.
For `input_mode: "spec"`, the kernel keeps the provided spec as the auditable
source and derives the target module/entry from `module_id` and the first
operation name or id. Browser clients should use this endpoint instead of
inventing their own connected-mode intent packet.

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

Run XTAL workflow request:

```http
POST /v1/sessions/{session_id}/xtal/run
```

The daemon derives safe binding variables from the approved intent packet. For
starter workspaces with no `x07.json`, it initializes an `xtal-pure` project,
then records visible operation records for `spec.scaffold`, `spec.check`,
`tests.gen.write`, `impl.sync.write`, `impl.check`, and `xtal.verify`.

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

Handoff prompts also define a structured agent event JSONL protocol. Agents may
emit one JSON object per line with
`schema_version: "x07.studio.agent_event@0.1.0"` and `kind` set to `artifact`,
`diagnostic`, `write`, or `approval`. The daemon records those as
`agent.event.<agent>.<kind>` operations with any safe artifact path attached,
which gives the browser approval and artifact signals without relying on
free-form terminal text.

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
