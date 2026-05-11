# XTAL workflow implementation findings

This document records friction found while implementing the Studio web surface.

## Findings

1. Studio needed a browser projection.

   The existing Rust GUI and TUI were useful thin clients, but the current
   product goal requires a browser-run XTAL surface for humans and agents. The
   new `web/` client keeps the daemon as source of truth and avoids creating a
   second lifecycle kernel.

2. The daemon has lifecycle events, but no text-to-intent endpoint.

   The web client can submit a complete `x07.studio.intent_packet@0.1.0`, but
   the daemon does not yet expose a first-class `intent.formalize` operation
   that accepts raw human text plus agent/provider choice. That makes polished
   intent generation a client concern for now. The next backend improvement is
   a daemon endpoint that records the raw plan, chosen agent, generated packet,
   revision history, and approval status as one auditable artifact.

3. Agent providers and coding-agent runners are different concepts.

   `ProviderProfile` is currently OpenAI-compatible model transport. That is
   enough for local/OAI-compatible inference, but OpenAI Codex and Claude Code
   need to be modeled as coding-agent runners with command/MCP capabilities,
   write scopes, and visible worklogs. The web UI shows both lanes, but the
   backend should add a `x07.studio.agent_profile@0.1.0` schema.

4. XTAL lifecycle commands are binding-first, but long-running visibility is
   still coarse.

   `OpRecord` persists completed command details. For a fully visible agent
   workflow, Studio needs streaming operation events: command started, stdout
   chunk, artifact detected, diagnostic classified, approval requested, and
   write completed.

5. Documentation is strong on agent quickstart but scattered for Studio.

   `x07/docs/getting-started/agent-quickstart.md`,
   `available-skills.md`, and the guides explain canonical loops, skills, and
   `x07 run`. Studio should compile those into session doctrine automatically
   and display the selected references in the session contract.

6. The API docs had a stale event envelope example.

   `DispatchEventRequest` wraps the tagged `SessionEvent` as
   `{ "event": { "event": "...", "payload": ... } }`. The older docs showed a
   flattened event/payload shape, which would slow down any browser or agent
   client integration. The example is now aligned with the Rust serde shape.
