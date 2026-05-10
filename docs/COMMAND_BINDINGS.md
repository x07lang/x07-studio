# Command bindings

Loom treats CLI execution as a rendered binding, not as free-form shell text.

## Implemented binding IDs in v0.1

### x07 XTAL / core

- `spec.scaffold`
- `spec.format`
- `spec.extract`
- `spec.lint`
- `spec.check`
- `tests.gen.write`
- `tests.gen.check`
- `impl.check`
- `impl.sync.write`
- `impl.sync.patchset`
- `xtal.dev`
- `xtal.verify`
- `xtal.repair`
- `xtal.certify`
- `xtal.ingest`
- `xtal.improve`
- `fmt.write`
- `lint.report`
- `fix.write`
- `patch.apply`
- `check.ast`
- `check.project`

### x07-wasm

- `wasm.web_ui.build`
- `wasm.web_ui.serve`
- `wasm.web_ui.test`
- `wasm.device.build`
- `wasm.device.verify`
- `wasm.device.package`
- `wasm.device.run.desktop_smoke`
- `wasm.device.provenance.attest`
- `wasm.device.provenance.verify`
- `wasm.workload.build`
- `wasm.workload.inspect`
- `wasm.topology.preview`
- `wasm.deploy.plan`

### x07-platform

- `lp.release.query`
- `lp.release.rollback`
- `lp.rollout.status`

## Machine-output policy

- `x07` and `x07-wasm` bindings run with `--json --report-out <file> --quiet-json` and capture the emitted report file.
- `x07lp` bindings run with `--json` and capture structured stdout/stderr directly.

## CLI override environment variables

- `X07_STUDIO_X07_EXE`
- `X07_STUDIO_X07_WASM_EXE`
- `X07_STUDIO_X07LP_EXE`
