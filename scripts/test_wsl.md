# Windows via WSL validation checklist

Run this from an Ubuntu WSL 2 distro with the repo checked out inside the Linux
filesystem, not under `/mnt/c`, to avoid file-watch and execute-bit drift.

## Prerequisites

- Rust toolchain can install the pinned `rust-toolchain.toml`.
- Node 22 and npm are available inside WSL.
- Playwright browser dependencies are installed:

```bash
cd web
npx playwright install --with-deps chromium
```

- `x07`, `x07-wasm`, and `x07lp` are on PATH or configured with:

```bash
export X07_STUDIO_X07_EXE=/path/to/x07
export X07_STUDIO_X07_WASM_EXE=/path/to/x07-wasm
export X07_STUDIO_X07LP_EXE=/path/to/x07lp
```

## Automated pass

```bash
./scripts/test_linux.sh
```

## Manual browser pass

1. Start the daemon:

   ```bash
   cargo run -p loom-daemon -- serve --root target/wsl-manual-workspace --addr 127.0.0.1:7719
   ```

2. Start the web UI:

   ```bash
   cd web
   LOOM_DAEMON_ORIGIN=http://127.0.0.1:7719 npm run dev -- --host 127.0.0.1 --port 5179
   ```

3. Open `http://127.0.0.1:5179` from the Windows browser.
4. Create a sorter or parser session.
5. Answer one clarification, build, verify, open TrustCard proof support, submit
   a local session summary, and refresh the certificate drawer.
6. Confirm `/v1/health`, `/v1/metrics`, and `python3 scripts/slo_dashboard.py`
   respond from the WSL shell.

## Known WSL caveats

- Keep workspaces under the WSL filesystem for predictable path permissions.
- Browser file watching is slower under `/mnt/c`.
- Windows browser access to WSL localhost depends on WSL networking mode; if
  localhost forwarding is disabled, use the WSL IP from `hostname -I`.
- Do not mix Windows and WSL Node package installs in the same `web/node_modules`
  directory.
