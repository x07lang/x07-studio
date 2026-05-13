# PBT flow

The verified turn can run property checks through the canonical x07 test path.

## Flow

1. Studio calls `x07 test --pbt --json --report-out <file> --quiet-json`.
2. The daemon projects the report into `x07.studio.pbt_round@0.1.0`.
3. Counterexamples show their repro ID, property, shrunk input, and repro path.
4. "Lock as regression test" calls
   `x07 fix --from-pbt <repro.json> --write` and then asks the kernel to
   re-run verification.

The PBT panel is intentionally inline on the verified turn because the action
belongs to the same trust review moment as proof evidence and runnable output.
