# Troubleshooting

## Session appears frozen

1. Check daemon health:

```bash
curl -sS http://127.0.0.1:7719/v1/health
```

2. If health responds, inspect the Process Lane and latest timeline turn.
3. If health does not respond, restart the daemon against the same workspace and refresh the browser.
4. If the issue repeats, capture a stress-pass snapshot and append a finding to `STRESS_PASS_FINDINGS.md`.

F3-class daemon freezes were fixed by the yielding autopilot and per-substep lock release. Current freezes should be treated as new findings, not the old F3 workaround.

## Wrong x07 binary is used

Set an explicit binary path:

```bash
X07_STUDIO_X07_EXE=$HOME/.x07/bin/x07 cargo run -p loom-daemon -- serve --root /path/to/project
```

See `V0_1_STATUS.md` for the full discovery order.

## Image upload rejected

Studio accepts PNG, JPEG, WebP, and GIF witnesses up to 4 MiB. HTML, SVG, executable, and unknown MIME types are rejected before they reach the workspace.

## AGENT.md save rejected

Reload the drawer if the file changed on disk. Studio also rejects empty AGENT.md bodies and bodies larger than 64 KiB.

## Trust posture stays pending

If a build or verify is running, TrustCard should show `Computing trust posture...`. If it stays pending after verification completes, check `/v1/sessions/{id}/trust/posture` and the latest `xtal.verify` operation.

## Useful local gates

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && npm run check && npm test && npm run build
npm run e2e
npm run e2e:connected
python3 scripts/check_no_metered_api.py
```
