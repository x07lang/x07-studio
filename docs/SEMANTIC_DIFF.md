# Semantic Diff

Semantic Diff is Studio's review lens for meaning-level changes. It is not a
text patch replacement; it answers whether the project became broader, riskier,
or less proved.

Endpoint:

```http
POST /v1/sessions/{session_id}/diff
```

Request refs can name:

- `current`
- `op_id`
- `turn_id`
- `hash`
- `quorum_proposal`

The response uses `x07.studio.semantic_diff@0.1.0` and includes a headline,
trust delta color, raw surfaces, and lists for world, capability, budget, and
proof changes. Browser compare panels and quorum proposal cards render this
same response shape.
