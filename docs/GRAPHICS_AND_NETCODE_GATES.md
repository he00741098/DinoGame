# Graphics and networking: production acceptance gates

This is an implementation roadmap, not a list of capabilities already provided by `engine/`.

**Campaign update:** protocol v3 adds twenty stages, authoritative mechanic state, controller-only restart/next, full-level revisioned snapshots, and neutral input rebaselining. Native now has an adaptive local-player camera, off-screen teammate markers, portrait/landscape control tests, and immediate path-change recovery hints. The [campaign catalogue](CAMPAIGN_LEVELS.md) records simulation completion coverage; these changes do not satisfy the remaining production gates by themselves.

**iOS-first milestone update:** an original, separately MIT-licensed Apple-only client now exists in [`native/`](../native/README.md), with SpriteKit presentation and live simulator-to-JavaScript-peer integration tests. It does not link the GPL Rust core. Protocol v2 now provides short-tap IDs, a 250 ms input lease, latest-value snapshots, generation-fenced replacement, and 15-second token-based resumption. These meet parts of the prototype boundary below, not the production graphics/prediction gates. Native physics sharing requires a rights-cleared permissive core or replacement; no license change to the legacy/core is assumed.

## Graphics: share the game, not necessarily the graphics API

**Initial production candidate:** SpriteKit on iOS and Pixi on web, both driven by the same Rust simulation through Swift/WASM bindings. Both provide mature GPU-backed 2D rendering without first building a custom platform stack. The existing Pixi assets/code may inform the web renderer, but its gameplay logic should not be imported.

**Alternative:** a shared Rust/wgpu renderer if native/web rendering parity, custom materials, or measured workload justify it. This is an evidence-based decision, not an irreversible commitment to SpriteKit. An arena fighter does not automatically require 3D or a different engine; clarify visual requirements before choosing that expansion.

### Presentation boundary to prove in the vertical slice

- Stable entity/resource identities; no renderer holding pointers into mutable simulation state.
- World-space transforms separate from camera, safe-area layout, and device pixels. Independent UI and world cameras; aspect-fit/letterbox behavior must not change gameplay visibility unfairly.
- Explicit layer ordering and batching rules; sprite atlases must preserve order for transparent overlap. Define premultiplied-alpha, color-space, nearest/linear sampling, and atlas-edge policies across both backends.
- Named animation clips and state transitions driven by simulation events. Animation never decides hitboxes, movement, or collision.
- Shared asset manifest and versioned identifiers. Load/decode off the render-critical path; bounded texture caches, missing-resource fallback, and controlled memory-pressure eviction.
- Debug overlays for authoritative/predicted/interpolated transforms, collision, frame time, snapshot age, RTT, and draw/resource counts.
- Presentation interpolation must not mutate the simulation. Reconciliation smooths a visual offset rather than gradually corrupting authoritative positions.
- Input sampling follows the device's frame rate; simulation/network cadence stays fixed. Touch, controller, and keyboard produce the same semantic inputs.
- Pause/resume, audio interruptions, resizing, rotation, screen scale, reduced motion, and accessibility are platform responsibilities, not simulation rules.

### Graphics go/no-go tests

1. Same replay renders recognizably equivalent gameplay on a real iPhone and a supported desktop/mobile browser. Pixel-identical UI is not required; collision and visibility are.
2. Select and document the minimum supported iPhone/browser before making performance promises. Target 60 fps and measure CPU/GPU frame-time distributions on that hardware, not just a simulator.
3. Test the worst intended level, four players, animation, effects, UI, and network activity together. Include sustained thermal/memory tests and asset reload/resume.
4. Instrument the renderer. Escalate to Metal/wgpu only with a failing requirement or measured bottleneck; do not build custom shaders and resource infrastructure merely to call the result an engine.

The current Canvas2D page intentionally fails these production gates. It is a transport/simulation diagnostic only.

## Networking: finish the timeline before adding visual prediction

The foundation uses authoritative simulation plus a bounded latest-held-input mailbox. That is sufficient to validate room isolation and gameplay authority, but it does not define a predictive client timeline.

### Next protocol milestone

1. Define fixed-tick input scheduling with bounded lead/lag relative to authoritative time. A client may suggest an input tick; it cannot move the server clock or force arbitrary catch-up work.
2. Preserve discrete actions (jump press/release) even when packets arrive together. Acknowledge exactly what was applied, not merely what arrived. Put explicit limits on command history and resimulation.
3. Implement client prediction using the shared Rust simulation, authoritative reconciliation, and replay of unacknowledged commands. Do not rewrite physics in JS or Swift.
4. Interpolate remote players/shared objects using bounded snapshot history and measured jitter. Decide how much shared-object prediction is tolerable for stacking before polishing the level design.
5. Add authenticated resumable session identities and expiry. Backgrounded iOS clients need neutral input, a grace period, and an explicit resume/full-state path. Room names do not grant identity or authority.
6. Replace the manual ruleset string with a reproducible content/rules manifest identity; mismatches fail before entering a room. Version binary codecs if measurements justify them.

### Network go/no-go tests

- Real iOS-to-browser cross-play with the same content and replay state semantics.
- Reproducible virtual-time harness for delay, jitter, stalls, disconnects, and resumption. Start with 100 ms RTT, ±30 ms jitter, and intermittent multi-second stalls; measure rather than promise correction-free interactions.
- For WebSocket, distinguish application-message coalescing from packet loss: TCP retransmits lost packets and can create head-of-line stalls. Redundant input messages do not remove TCP head-of-line blocking.
- Malformed, duplicate, stale, future, oversized, and flooding inputs never crash a room, advance its clock, or grow queues without bound.
- Slow/offline clients cannot stall other rooms; overload behavior, room limits, and latency/memory budgets are measured under load.
- Authentication/authorization, WSS termination, browser origin policy, per-source admission limits, logs/metrics, deployment health, and shutdown/drain behavior are reviewed before public access.

### Fighter expansion gate

Before claiming Smash-like support, demonstrate bounded rollback and resimulation, complete state capture (including RNG and game-specific state), runtime hash parity on ARM/WASM/server, rollback-safe presentation events, and measured input-to-photon latency. Re-evaluate transport then; shipping co-op over WebSocket is not proof that the same transport will meet fighting-game requirements.
