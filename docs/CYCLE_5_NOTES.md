# Cycle 5 notes

Cycle 5 makes Studio speak the canonical x07 agent loop directly. The daemon
now wraps the practical tools from `x07/docs/getting-started/agent-workflow.md`:
`AGENT.md`, `x07 lint`, `x07 fix`, `x07 doctor`, `x07 pkg lock --check`,
`x07 migrate`, `x07 project migrate`, `x07 test --pbt`, `x07 arch check`, and
`x07 pkg provides`.

## Wired

- `AGENT.md` is exposed as `x07.studio.agent_contract@0.1.0`, editable from the
  header drawer, and folded into clarify/realize handoff prompts.
- `x07 lint` runs after the build pipeline and projects into a compact Lint
  turn. `x07 fix` quickfixes reuse the Cycle 4 quickfix review shape with
  optional before/after snippets.
- HealthRow summarizes `doctor`, package lock, and migration state above the
  TrustCard.
- PBT runs from the verified turn and can lock counterexamples as regression
  tests through `x07 fix --from-pbt`.
- Shareable, Team, and Production ladder rungs include `arch-check`; Shareable
  also includes a lockfile gate.
- Ask-the-project and ModuleSearch call `x07 pkg provides` for module IDs such
  as `text.normalize_v1`.
- The welcome surface uses the ten canonical agent-gate recipes from the x07
  agent workflow doc.
- TrustCard is promoted to the right-rail hero, secondary tools move into
  DrawerRail, posture turns collapse into PostureBadge, and compare actions are
  hidden behind CompareMenu.

## Verification

Cycle 5 adds Rust unit coverage for the new projections, a canonical recipe
drift gate, Vitest coverage for recipes/motion/DrawerRail, and connected specs
`zr` through `zu` for AGENT.md, lint, health, and arch-check.
