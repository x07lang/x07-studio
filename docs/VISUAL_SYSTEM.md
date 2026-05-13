# Visual system

Cycle 4 keeps the visual language small and operational:

- posture colors: green for pure/proved, amber for bounded widening, red for
  risky widening or proof drops
- stable spacing and radius tokens in `web/src/lib/styles/tokens.css`
- restrained motion tokens for optimistic UI and panel transitions
- mono text for commands, file paths, signatures, and schema ids
- cards only for repeated items, drawers, panels, and concrete review tools

Primary surfaces:

- Trust Card
- Semantic Diff
- Proof Explorer
- Quickfix Card
- Cassette Ribbon
- Shipping Ladder gates
- Certificate View
- Command Palette
- Welcome recipes

These components should render lifecycle evidence, not instructional copy. New
controls should attach to daemon operations or typed API projections before
they become visible actions.
