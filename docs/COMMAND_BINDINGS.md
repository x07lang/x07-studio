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
- `trust.report.sandbox`
- `trust.profile.check`
- `trust.certify.profile`
- `fmt.write`
- `lint.report`
- `fix.write`
- `patch.apply`
- `check.ast`
- `check.project`
- `pkg.lock.atlas.frontend`

`xtal.verify` accepts the Studio verify run variables `proof_policy`,
`allow_os_world`, `unwind`, `max_bytes_len`, and `input_len_bytes`. Loom
validates those values before rendering them as `x07 xtal verify` flags, so the
browser proof/world/bounds controls change the canonical command rather than
only decorating the review surface.

`xtal.repair` accepts the Studio repair variables `repair_entry`,
`repair_strategy`, `repair_write`, `repair_max_rounds`,
`repair_max_candidates`, `repair_semantic_max_depth`, and
`repair_allow_edit_non_stubs`. Loom validates those values before rendering
them as `x07 xtal repair` flags, so the browser repair-room controls change the
canonical repair command rather than only decorating the Counterexample Theater.

`xtal.certify` accepts the Studio certify variables `cert_spec_dir`,
`cert_entry`, `cert_all`, and `cert_no_prechecks`. Loom validates those values
before rendering them as `x07 xtal certify` flags, and `cert_all=true`
suppresses the entry flag so Studio does not render conflicting certification
scope.

### x07-wasm

- `wasm.app.profile.validate.atlas_dev`
- `wasm.app.contracts.validate`
- `wasm.app.build.atlas_dev`
- `wasm.app.serve.smoke.atlas_dev`
- `wasm.app.test.happy_path`
- `wasm.app.test.validation_error`
- `wasm.app.test.regress.atlas_incident`
- `wasm.app.build.atlas_release`
- `wasm.app.pack.atlas_release`
- `wasm.app.verify.atlas_release`
- `wasm.web_ui.build`
- `wasm.web_ui.serve`
- `wasm.web_ui.test`
- `wasm.web_ui.contracts.validate`
- `wasm.http.contracts.validate`
- `wasm.caps.validate.atlas_release`
- `wasm.ops.validate`
- `wasm.slo.validate.atlas`
- `wasm.slo.eval.atlas_canary_ok`
- `wasm.provenance.attest.atlas_release`
- `wasm.provenance.verify.atlas_release`
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
- `wasm.deploy.plan.atlas_release`

### x07-platform

- `lp.release.query`
- `lp.release.rollback`
- `lp.deploy.accept.local`
- `lp.deploy.run.local`
- `lp.deploy.run.local.metrics`
- `lp.deploy.query.local`
- `lp.deploy.status.local`
- `lp.incident.list.local`
- `lp.regress.from_incident.local`
- `lp.ui.serve.local`

The local deployment bindings target the current `x07lp` driver surface:
`accept`, `run`, `query`, `status`, `incident-list`, `regress-from-incident`,
and `ui-serve`. They are intended to sit after `wasm.app.pack`,
`wasm.app.verify`, and `wasm.deploy.plan` in an end-to-end Studio lane.
Studio's Atlas workflow renders absolute `*_arg` command paths for `x07lp`
while recording relative artifacts such as `.x07/platform` in the session log,
so direct source checkouts of `x07-platform/scripts/x07lp-driver` can consume
project artifacts without breaking Studio artifact previews.

## Machine-output policy

- `x07` and `x07-wasm` bindings run with `--json --report-out <file> --quiet-json` and capture the emitted report file.
- `x07lp` bindings run with `--json` and capture structured stdout/stderr directly.

## Cycle 5 canonical-loop wrappers

Some Cycle 5 commands are rendered from endpoint-specific wrappers because
their arguments depend on the current diagnostic, repro, or module ID rather
than the static binding catalog:

- `x07 doctor`
- `x07 pkg lock --project x07.json --check`
- `x07 migrate --check/--write --to 0.5`
- `x07 project migrate --check/--write --project x07.json`
- `x07 lint --input <file.x07.json>`
- `x07 fix --diagnostic <id>` or `x07 fix --input <file> --write`
- `x07 test --pbt`
- `x07 fix --from-pbt <repro.json> --write`
- `x07 arch check`
- `x07 pkg provides <module-id> --project x07.json`

They still use the same `CliAdapter` execution path and the same machine-output
policy as catalog bindings.

## CLI override environment variables

- `X07_STUDIO_X07_EXE`
- `X07_STUDIO_X07_WASM_EXE`
- `X07_STUDIO_X07LP_EXE`

If `X07_STUDIO_X07LP_EXE` is unset, Studio looks for a sibling
`x07-platform/scripts/x07lp-driver` checkout before falling back to `x07lp` on
`PATH`.
