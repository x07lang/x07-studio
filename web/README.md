# x07 Studio Web

SvelteKit projection for the Loom daemon.

It renders the XTAL lifecycle as an approval-gated operating surface:

```text
intent -> spec -> realization -> verify -> repair -> trust/certify -> ops
```

The web client uses the real daemon endpoints under `/v1/**` when `loom-daemon`
is running. If the daemon is unavailable, it falls back to a deterministic demo
projection so the UX and browser tests still exercise the same phase model.

The intake form includes simple, intermediate, and complex project briefs. Each
brief can be edited before creating a Studio session, then polished into an
intent packet and driven through the approval-gated XTAL loop.

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
```
