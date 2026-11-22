# Multiplayer engine direction

## iOS-first update (2026-11-21)

The user's latest priorities are native iOS functionality, robustness, and avoiding licensing-problematic dependencies. [`native/`](../native/README.md) now provides an independent, original MIT Apple-framework-only client. It speaks the engine protocol but does not link the GPL Rust simulation. **The shared Rust/Swift prediction plan below is conditional on resolving production-core licensing/provenance or replacing that implementation; it is not permission to bundle GPL code into iOS.** The legacy/root and engine licenses remain unchanged.

LAN/hosted/P2P connectivity should be routes/adapters of one session system, not independent game implementations. Current native support is manual LAN/hosted WebSocket endpoints, retry/fallback, and bounded token-based resumption. P2P, automatic discovery, NAT traversal, and authority migration are not implemented or promised. Simulator integration is verified; physical-device profiling remains a gate.

## Product constraints

- First shipping target: iOS; browser cross-play must not require a separate game simulation.
- First game: an original, small-session cooperative platformer inspired by the teamwork of Pico Park, not a clone of its art or levels.
- Later games may include arena fighters. Reuse the platform, but do not promise that platformer networking alone solves competitive fighting games.
- Graphics and networking are core capabilities, not game-specific utilities.
- Preserve the Dino implementations as migration references rather than rewriting them in place.

## Boundaries

1. **Simulation:** fixed-step, renderer-independent Rust world state, stable entity identities, explicit inputs, reproducible state transitions, and game-specific rules outside the transport.
2. **Session runtime:** authoritative rooms, bounded input queues, protocol/version negotiation, join/leave lifecycle, rate limits, and snapshots. A client never submits positions or decides completion.
3. **Client runtime:** input sampling, prediction/reconciliation, remote interpolation, presentation time, connection recovery, and platform lifecycle handling. These are distinct from simulation time.
4. **Graphics:** renderer-independent presentation data with stable IDs, cameras, sprite instances, layers, animation, and resource handles. Start production evaluation with SpriteKit on iOS and Pixi on web; keep game rules out of both. Evaluate a shared Rust/wgpu backend if profiling or custom effects justify its platform integration costs. Keep a simple diagnostic renderer separate from these production backends.
5. **Platform:** Swift/iOS shell and browser shell supply touch/controller input, audio, assets, accessibility, lifecycle, and transport adapters. No platform APIs in simulation.
6. **Game:** levels, interactions, objectives, and game-specific content. Start with movement, cooperative stacking, a key, and a shared exit.

## Networking principles

- Authoritative fixed simulation ticks, not wall-clock deltas received from clients.
- Versioned messages; reject incompatible clients before joining a room.
- Input sequence numbers and snapshot acknowledgements; clients cannot advance the authoritative clock.
- Reliable WebSockets are an accessible first transport, not a claim of fighting-game-quality latency. Transport remains replaceable.
- Bound rooms, peers, messages, and queued work; remove disconnected players.
- No client-controlled server filesystem paths, asset URLs, or arbitrary serialized game code.
- An arena fighter requires a separate rollback/resimulation prototype, determinism validation across targets, and latency testing before selecting its shipping transport.

## Delivery gates

1. **Foundation:** testable simulation, versioned protocol, authoritative room host, browser development harness, and documented migration baseline.
2. **Playable iOS vertical slice:** shared core linked into an iOS app, touch/controller controls, GPU-backed rendering, real-device profiling, browser cross-play, prediction/interpolation, and simulated poor-network tests.
3. **Co-op production:** level tools, content pipeline, lobby/rejoin UX, authentication/invites, deployment, observability, abuse protections, accessibility, and background/resume policy.
4. **Engine expansion:** a second game demonstrates reuse; evaluate rollback and richer animation/effects with actual arena-fighter requirements.

Nothing in the foundation milestone should be described as an App Store-ready engine. In particular, a browser canvas preview is a test harness, not the promised production graphics system.

Detailed acceptance criteria: [graphics and netcode gates](GRAPHICS_AND_NETCODE_GATES.md).
