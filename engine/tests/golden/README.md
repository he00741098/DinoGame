# Co-op replay v1

`coop-v1.hash` is the final canonical world checksum for the 563-tick, two-player input-only replay in `sim-core::tests::two_players_can_complete_the_level_without_teleporting`:

1. Player 2 moves right for 200 ticks to become a step below the gate.
2. Player 1 approaches for 52 ticks and jumps right for 50 ticks to stand on player 2.
3. Player 1 releases jump for one tick, then jumps right for 60 ticks to collect the key and retract the gate.
4. Both move toward the exit for 200 ticks, stopping at the exit target or on completion.

No positions are injected. The test asserts stacking, key collection, shared completion, and the final checksum. A separate 3,600-tick test compares identical replays at every tick with reversed command ordering.

The campaign milestone upgrades the canonical world-state encoding to **version 2**, including mechanic objects, timers, deaths, checkpoint, and level-start tick. The reviewed checksum is now `3395276329381061516`; the original replay's gameplay is unchanged. This prevents new authoritative fields from being silently omitted merely to retain an older checksum.

Changes to the replay hash require reviewing the physics/rules change and versioning the ruleset when incompatible. This fixture is a host-side regression gate, not proof of cross-target runtime parity. Linux/macOS CI is configured; iOS/WASM replay execution is not yet implemented.
