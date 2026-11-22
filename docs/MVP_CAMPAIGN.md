# Co-op MVP: 20 stages, one connected team

## Scope

An original 2–4 player co-op platformer for iOS, with browser cross-play. Pico Park is a reference for cooperative problem solving, not a source of art, level layouts, names, or code.

The MVP target is **20 playable stages of increasing complexity**, not twenty recolored versions of one level. Introduce mechanics with forgiving spaces, then combine them. Levels must remain completable by two players; four players must not create impossible spawn/exit arrangements.

## Fifteen mechanic families

1. Teammate stacking / human steps.
2. Shared key and retracting key barrier.
3. Everyone-at-the-exit team completion.
4. Pressure plates controlling a gate.
5. Periodic timed gates.
6. One-way ledges.
7. Moving platforms and rider transport.
8. Conveyors.
9. Slippery ice.
10. Spring pads.
11. Hazards and coordinated retry.
12. Checkpoints.
13. Crumbling platforms.
14. Low-gravity areas.
15. Wind areas.

Pressure plates and their paired gate count as **one** mechanic family. UI, animations, and alternative colors are not additional mechanics.

## Session and progression rules

- Each room owns its current stage, world, and a monotonically increasing revision.
- Snapshots include the complete current level and revision. A latest-value snapshot mailbox cannot lose a separate “level changed” event.
- The lowest connected player ID is the current room controller. Only that player can restart or advance; advance requires authoritative completion.
- Repeated, old-revision, old-generation, non-controller, and premature transition requests cannot reset/skip stages.
- Transitions preserve player identities and network ticks. Inputs are neutralized, then each client establishes a new-revision neutral baseline before controls reactivate. An in-flight jump from the previous stage cannot become a jump in the next stage.
- Stage 20 has a final campaign-complete state; no stage 21 is created.
- A disconnected participant cannot trigger a new shared win. Resume restores the current room stage, not the stage in an old welcome packet.

## Connectivity direction

Use one session runtime with replaceable routes/transports. The current route set is LAN/hosted WebSocket endpoints with token-based resume and bounded retry. Native network-path changes should wake recovery immediately, and a working fallback should remain preferred after background/foreground recovery.

This does **not** yet provide redundant authorities, P2P/NAT traversal, replicated room storage, or seamless migration between arbitrary hosts. A route to a different host may be a different world. Never leak a resume credential to an unverified alternative endpoint merely to make failover appear seamless.

## MVP acceptance gates

- Campaign catalogue validates all twenty stages, bounded geometry/state, unique identities, and feature coverage.
- Each mechanic has an authoritative regression test; interactions receive deterministic replay / no-penetration / respawn safety tests.
- All **twenty** stages now have death-free, two-player, input-only completion witnesses. This proves simulation solvability, not human difficulty, touch usability, or fun; those still require hands-on review before calling the campaign balanced or release-ready.
- iOS renders every active mechanic and the current objective, exposes controller-only restart/next, and keeps controls usable with safe areas and rotation.
- Real simulator-to-JavaScript-peer integration checks version compatibility, movement, short taps, resume, and route fallback. Real iPhone Wi-Fi/cellular and poor-network tests remain necessary.
- Native dependencies remain Apple SDK-only. The separate GPL development host/core is not linked into the MIT native app. Production shared-core licensing must be resolved or that implementation replaced before native prediction/shipping.

The campaign is an MVP candidate until these gates are met; a catalogue or passing parser test alone is not proof of twenty fun, solvable levels.

See the [20-stage catalogue and play instructions](CAMPAIGN_LEVELS.md).
