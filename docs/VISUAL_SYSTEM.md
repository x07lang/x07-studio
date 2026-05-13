# Visual system

Cycle 4 keeps the visual language small and operational. Cycle 5 keeps that
constraint while making the TrustCard the right-rail hero:

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
- HealthRow
- DrawerRail
- PostureBadge
- CompareMenu
- QuickfixThreePane
- PbtPanel
- ModuleSearch

Cycle 5 layout rules:

- HealthRow stays compact and above the TrustCard.
- TrustCard is the largest right-rail surface and carries the posture color
  band.
- Secondary tools live in DrawerRail instead of competing with the TrustCard.
- PostureBadge replaces duplicate trust posture turns.
- CompareMenu is a hover/click affordance; the timeline should not show a
  permanent compare button on every turn.
- QuickfixThreePane shows before, operation, and after panes instead of raw JSON
  as the primary review surface.

Motion is only for state transitions: posture color, drawer expansion, quickfix
apply pulse, and semantic-diff entry. It should never move timeline content
continuously.

These components should render lifecycle evidence, not instructional copy. New
controls should attach to daemon operations or typed API projections before
they become visible actions.
