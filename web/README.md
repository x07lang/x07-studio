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
The main timeline renders typed turns from the daemon, while the side panels
cover Try-It invocation, shipping ladder state, incidents, cassette history,
project Q&A, sync codes, local memory, release status, replay export, and
visual canvas editors. Realize runs stream normalized Claude Code / Codex tool
events into the timeline; Compare both agents runs a side-by-side quorum diff
before applying a chosen proposal.

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
