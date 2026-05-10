# x07-studio agent doctrine

This repo is a lifecycle surface for x07 and XTAL. Agents operating here must remain artifact-first and spec-first.

## Required flow

1. Create or update an intent packet.
2. Convert intent into spec artifacts and examples.
3. Only then synchronize realization.
4. Run verify.
5. If verify fails, prefer semantic or quickfix repair through canonical x07 commands.
6. Route spec-changing repairs back to spec review.
7. Only certify from evidence-backed trust review.
8. Only improve from structured incidents or runtime artifacts.

## Do not do these things

- Do not bypass the canonical x07 CLI / MCP / platform contracts.
- Do not turn natural language directly into unchecked code.
- Do not widen worlds, capabilities, budgets, or sandbox policy without surfacing it for review.
- Do not scrape help text when machine-readable outputs exist.

## Canonical machine surfaces

- x07 / x07-wasm / x07lp `--json` + `--report-out`
- x07 `--cli-specrows`
- MCP JSON-RPC
- XTAL artifacts under `target/xtal/`
- Studio-local artifacts under `.x07/studio/`
