# Process Lane

The Process Lane is a projection over the session operation log. It does not
own state and it does not invent steps outside the canonical x07 loop.

Canonical step order:

```text
intent -> agent_md -> clarify -> spec -> tests -> impl -> verify -> prove
-> lint -> review -> repair -> arch_check -> lockfile -> migrate -> pbt
-> ladder_climb -> certify
```

Each `CanonicalStep` carries actor, status, timing, op backlink, narration,
optional step budget, and optional review round. The browser renders the lane
above the Timeline so the user can see past, current, and next work before
reading detailed turns.

Actor colors:

- Conductor: Studio deterministic operations, cyan.
- Architect: spec, AGENT.md, and policy judgment, mint.
- Coder: implementation and repair work, amber.
- Reviewer: review rounds, violet.

Clicking a step opens the Step Drawer for the linked `OpRecord`, stream events,
and artifacts. Hovering a pending step calls the what-if endpoint.
