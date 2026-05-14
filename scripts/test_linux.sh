#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo}"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_no_metered_api.py
python3 -m unittest discover scripts 'test_*.py'

(
  cd web
  npm ci
  npm run check
  npm test -- --run
  npm run build
  npm run e2e
  npm run e2e:connected
)

python3 scripts/a11y_audit.py
python3 scripts/perf_budget.py
cargo build --release -p loom-daemon -p x07-studio -p x07-studio-forge
python3 scripts/package_standalone.py --target-dir target/release --web-dir web/build --out-dir dist/standalone-smoke
python3 scripts/validate_standalone_bundle.py --dist-dir dist/standalone-smoke
python3 scripts/smoke_standalone_launcher.py --dist-dir dist/standalone-smoke
