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

## Bindings

- `GET /bindings`

Returns the canonical rendered binding catalog exposed by `loom-adapters`.

## Sessions

- `GET /sessions`
- `POST /sessions`
- `GET /sessions/{session_id}`
- `POST /sessions/{session_id}/events`
- `POST /sessions/{session_id}/bindings/run`
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

Run binding request:

```json
{
  "binding_id": "spec.check",
  "vars": {"input": "spec/app.sorter.x07spec.json"}
}
```

Run XTAL workflow request:

```http
POST /v1/sessions/{session_id}/xtal/run
```

The daemon derives safe binding variables from the approved intent packet. If
the workspace has no `x07.json`, it initializes an `xtal-pure` project, then
runs visible operation records for `spec.scaffold`, `spec.check`,
`tests.gen.write`, `impl.sync.write`, `impl.check`, and `xtal.verify`.

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
while the command is active, then updates the same operation with captured
stdout/stderr and a succeeded or failed status.

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
