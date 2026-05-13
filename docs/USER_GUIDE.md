# User guide

## Install and start

1. Put `x07-studio` beside the x07 ecosystem repos when developing locally.
2. Ensure required CLIs are available, or set explicit paths:
   - `X07_STUDIO_X07_EXE`
   - `X07_STUDIO_X07_WASM_EXE`
   - `X07_STUDIO_X07LP_EXE`
3. Start the daemon and web shell:

```bash
cargo run -p loom-daemon -- serve --root /path/to/x07/project --addr 127.0.0.1:7719
cd web
npm run dev
```

## First session

1. Enter the behavior you want in the composer.
2. Review the clarified intent and assumptions.
3. Approve the build.
4. Watch the Process Lane for the active x07 step and responsible actor.
5. Review the verified summary, TrustCard, and Shipping Ladder.
6. Use Try It only after the session reaches a verified/trust-review state.

Studio is spec-first. Natural language becomes a reviewable intent packet, then a spec, then implementation and evidence.

## Simple vs Details

The default view keeps the current room, agent worklog, Process Lane, TrustCard, and Shipping Ladder visible. Details mode exposes the deeper audit surface: command bindings, evidence boards, role routing, replay, visual editors, and package/arch/PBT panels.

## Shipping rungs

| Rung | Meaning |
|---|---|
| Local preview | Verified locally with runnable evidence. |
| Shareable | Adds architecture and lockfile gates. |
| Team | Adds certificate review. |
| Production | Requires the public-beta/GA readiness gates in `PRODUCTION_READINESS.md`. |

## Verify failures

When verify fails, read the failing turn first. If a quickfix is available, open the diagnostic card and apply it. If the failure changes behavior, return to the intent/spec review before accepting the repair.

## Toolchain selection

Set `X07_STUDIO_X07_EXE=$HOME/.x07/bin/x07` when you want Studio to use the installed x07 toolchain instead of sibling development builds. See `V0_1_STATUS.md` for the full discovery order.
