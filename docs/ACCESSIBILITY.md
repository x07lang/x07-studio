# Accessibility baseline

Last updated 2026-05-13 for the pre-production readiness pass.

## Current baseline

- Primary commands are native `<button>` elements.
- The command palette is exposed as a modal dialog with an accessible input label.
- The Process Lane announces current-step changes with `aria-live="polite"`.
- Global `:focus-visible` styling gives keyboard users a visible 2px focus ring.
- Motion helpers no-op when `prefers-reduced-motion: reduce` is active.
- The TrustCard proof-support panel uses native `<details>/<summary>`.

## Keyboard map

| Surface | Key |
|---|---|
| Composer submit | `Cmd/Ctrl+Enter` |
| Command palette | `Cmd/Ctrl+K` |
| Command palette navigation | `ArrowUp`, `ArrowDown`, `Enter`, `Escape` |
| XTAL flow | Tab through native buttons, selects, and text areas |

## Known limitations

- A full axe-core serious/critical audit is still pending because `@axe-core/playwright` is not currently part of the web dependency set.
- Web Speech is browser-dependent and unavailable in some headless WebKit/Safari contexts; the text composer remains the accessible fallback.
- The visual canvas editor has keyboard-accessible form fields for labels and emit/parse controls, but graph edge manipulation is still pointer-oriented.

## Verification

Manual keyboard-only validation should cover:

1. Create a session from the composer.
2. Answer a clarification question.
3. Approve and build.
4. Open TrustCard proof support.
5. Open the command palette and run a command.
