# Cycle 4 notes

Cycle 4 moves Studio from "can run the lifecycle" to "can explain why a
lifecycle step is safe." The browser surfaces are still daemon projections over
operation records and workspace artifacts.

## Wired

- Compatibility fixes for `/v1/sessions`, `realize/quorum`, and
  `ladder/release` keep older web clients working while the preferred API shape
  stays explicit.
- The Trust Card summarizes worlds, capability reads, budgets, proof coverage,
  posture color, and posture deltas from `posture.captured` operations.
- Semantic Diff compares current state, operations, timeline turns, hashes, and
  quorum proposals across world, capability, budget, and proof dimensions.
- Proof Explorer links plain-English behavior promises to verify/proof
  evidence and assumptions.
- Quickfix records turn incident bundles and latest repair patchsets into a
  reviewable AST preview.
- Cassette Ribbon projects `.x07_rr` replay boundaries in chronological order.
- Shipping Ladder rungs now include named gates, and successful profile
  certification satisfies the matching rung even when the profile file lives
  outside the project.
- Certificate view reads certificate, trust, and verify artifacts and can
  refresh by running the certify lane.
- The command palette and empty-workspace recipes give fast entry points without
  creating hidden browser-only lifecycle state.

## Verification

Cycle 4 adds browser coverage for welcome recipes, command palette, trust
posture, semantic diff, rung gates, certificate view, and quickfix records. The
connected specs run against the real Loom daemon with the deterministic local
toolchain under `scripts/serve_connected_e2e_daemon.py`.
