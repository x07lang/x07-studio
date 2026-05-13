# Lint loop

Studio runs the explicit x07 diagnostic path instead of inventing browser-only
lint state.

## Flow

1. Build/verify completes.
2. The kernel runs `x07 lint --project x07.json --json --report-out <file> --quiet-json`.
3. Diagnostics are projected into `x07.studio.lint_report@0.1.0`.
4. The timeline shows a compact Lint turn with severity counts and diagnostic
   IDs.
5. The Lint drawer applies `x07 fix` through the daemon and refreshes lint.

Quickfix results reuse `x07.studio.quickfix_record@0.1.0`, including optional
`before_snippet` and `after_snippet` fields when Studio can materialize the
patch against a cited workspace file.
