# x07 Studio Web

SvelteKit projection for the Loom daemon.

It renders the XTAL lifecycle as a Timeline operating surface:

```text
intent -> clarify -> approve -> build -> verify -> try -> ship -> learn
```

The web client uses the real daemon endpoints under `/v1/**` when `loom-daemon`
is running. If the daemon is unavailable, it falls back to a deterministic demo
projection so the UX and browser tests still exercise the same phase model.

The composer accepts text intent, voice transcripts, image witnesses, Auto mode,
and `/binding <id>` commands.
The Process Lane above the timeline shows the current canonical step, actor,
next step, budget, what-if forecast, and click-through evidence. The main
timeline renders typed turns from the daemon, while the side panels cover Try-It
invocation, trust posture, shipping ladder gates, incidents, quickfix records,
cassette boundary ribbons, project Q&A, sync codes, local memory, role
overrides, release status, certificate review, replay export, visual canvas
editors, AGENT.md editing, lint quickfixes, health checks, PBT counterexamples,
arch-check gates, and package discovery. Realize runs stream normalized Claude
Code / Codex tool and MCP events into the timeline; Second opinion runs the
side-by-side quorum diff only when the user asks for it. Cycle 4 also adds a
command palette (`Cmd/Ctrl+K`); Cycle 5 replaces invented recipe cards with the
ten canonical x07 agent-gate recipes; Cycle 6 adds role-based collaboration.

## Pointing Vite at a non-default daemon

The Svelte dev server proxies `/v1/**` to `LOOM_DAEMON_ORIGIN`. Use it when the
daemon is not on `127.0.0.1:7719`:

```bash
LOOM_DAEMON_ORIGIN=http://127.0.0.1:7729 npm run dev
```

## Local use

```bash
cargo run -p loom-daemon -- serve --root /path/to/x07/project --addr 127.0.0.1:7719
cd web
npm install
npm run dev
```

## Validation

```bash
npm run check
npm test
npm run build
npm run e2e
npm run e2e:connected
```
