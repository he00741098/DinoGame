# iOS-first co-op client

Native iOS 17+ MVP candidate using **SwiftUI, UIKit, SpriteKit, Network, and Foundation WebSockets**. See the [campaign catalogue](../docs/CAMPAIGN_LEVELS.md) for the correctly staffed input-only completion witnesses and expedition requirements. There are no third-party packages, web views, Rust libraries, or GPL code linked into this app. The original code under this directory has its own [MIT license](LICENSE); this does **not** relicense `engine/` or the legacy repository.

## Run on iOS

Open `native/CoopClient.xcodeproj`, choose the `CoopClient` scheme and an iPhone simulator, and Run.

Start the separate development host:

```sh
cd engine
cargo run --locked -p multiplayer-server
```

- **Simulator:** join `ws://127.0.0.1:3000`.
- **Physical iPhone:** select your signing team, start the host with `BIND_ADDR=0.0.0.0:3000`, enter `ws://<Mac-private-LAN-IP>:3000`, and allow Local Network access. Loopback on an iPhone means the phone itself, not the Mac.
- **Hosted route:** use `wss://<host>/ws`. No broad ATS bypass is enabled. Insecure `ws://` is restricted to private/local addresses in **Debug** builds; Release requires WSS.

Use the same room on a browser or another iPhone. Hold left/right; tap jump. Multiple controls can be held together. The lowest connected player ID controls Restart; see [exit and automatic progression rules](../engine/README.md#exit-review-and-network-diagnostics). A camera follows the local player on long stages and marks off-screen teammates; all mechanics use authoritative positions/state with distinct visual cues. Native input samples at approximately 59 Hz, sending changes or a 100 ms heartbeat; authority runs at 60 Hz, snapshots at 20 Hz. SpriteKit presents snapshots with simple 50 ms visual smoothing; **there is no local physics/prediction yet**. See [the campaign specification](../docs/MVP_CAMPAIGN.md) for the feature roster and readiness gates.

## One session, multiple routes

The primary and optional fallback are addresses for one common protocol, not separate game modes. Routes currently use WebSockets for both LAN and hosted play. Retry policy, input state, version negotiation, snapshot validation, and resume handling are shared.

- Fresh healthy connections reset failure backoff and are retried first. Failed attempts rotate routes with capped jittered delays.
- The working route is remembered across background/foreground recovery. Network framework path-change hints wake recovery immediately on loss/restoration or Wi-Fi/cellular route changes instead of always waiting for a socket timeout. Duplicate hints do not repeatedly restart a connection; hints never permanently block LAN/loopback because they are not proof of server reachability.
- Resume tokens stay in memory, scoped to the exact endpoint and room. They are never put in URLs, preferences, or logs, nor forwarded to an unrelated fallback. **Two arbitrary servers do not share a world or session store.** Fallback to a different host can create a different session; it is not seamless authority migration.
- Background/inactive scenes release input and cancel the transport. Foregrounding attempts the same session. The development server retains a disconnected player for 15 seconds; longer absence explicitly requires a fresh join. Terminating the app loses the memory-only token.
- The server expires stale input after 250 ms even if iOS never delivers a final release. Old sockets cannot apply commands or delete a replacement connection.
- Stage changes are revision-fenced. Controls clear, establish a neutral input baseline, and wait for its acknowledgment before reactivating; an in-flight jump cannot spill into a restarted/next level. All snapshots include the current full stage, so reconnect never restores obsolete geometry.
- Without fresh snapshots, controls pause after 750 ms; after five seconds the socket is replaced. Received duplicates do not renew snapshot freshness.
- Input acknowledgment progress has an independent three-second deadline. Fresh snapshots cannot hide a server that never acknowledges offered input, and repeated heartbeats do not extend that deadline. A route is considered healthy for retry preference only after controls actually synchronize, not just after it sends a world snapshot.
- Pending short taps survive timer sampling, but pausing discards **unsent** actions without regressing already transmitted jump IDs. Controls handle touch release, drag-out, cancellation, disable, and VoiceOver activation.
- Only validated, bounded snapshots reach the renderer. Old connection generations and old ticks cannot overwrite newer state. Welcome must precede snapshots; impossible dimensions, IDs, acknowledgements, and protocol versions stop safely.

P2P, NAT traversal, automatic Bonjour discovery, relay selection, host migration, and persistent identity are **not implemented**. Future adapters should reuse this session boundary rather than create separate game/lobby systems. Connectivity cannot be guaranteed across all networks; reconnection and explicit failure UX remain required even with multiple routes.

## Test

Portable policy/parser tests on macOS:

```sh
cd native
swift test --scratch-path "${TMPDIR%/}/coop-swift-tests"
```

Scratch output outside a synced Documents folder avoids macOS signing failures from Finder/resource-fork metadata. The tests cover route validation, capped retry history, short taps, pause cancellation, input sources, scoped credentials, snapshot ordering/acks, and malformed/hostile data.

Reproducible **real iOS simulator + JavaScript peer + authoritative server** integration:

```sh
python3 native/scripts/test_ios.py
# Optional: --device <simulator-UUID> --artifacts <output-directory>
```

Requires macOS/Xcode, an installed iPhone simulator, Rust 1.94, and Node 22+. This first checks the bounded fault-proxy codec, exports all twenty real server level/snapshot fixtures, and runs the Swift parser/policy tests. It then starts an isolated loopback host, JavaScript peer, and acknowledgment-stripping proxy and runs **five XCTest UI tests**. It checks the result bundle and stops only the child processes it started. UI tests exercise real Foundation WebSockets, cross-play, movement, a short jump, background/resume identity, invalid endpoints, failed-primary fallback, controller restart/rebaselining, and recovery when snapshots stay fresh but input acknowledgments disappear. The proxy must strip at least twenty snapshots containing real applied-input acks and keep the fault connection alive for at least 2.5 seconds, so an early proxy disconnect cannot falsely pass the recovery test. The `rises` diagnostic counts **authoritative upward-velocity transitions**, so a short jump remains observable after XCTest waits for UI idleness.

Verified locally: iPhone 16e / iOS 18.6 simulator; Release ARM64 device build without signing. **Physical-device execution, Wi-Fi/cellular switching, real LAN discovery, battery/thermal profiling, and App Store release are not yet verified.**

## Shipping / licensing gates

- Native app sources are separate original MIT code and depend only on Apple SDKs. New third-party dependencies require explicit license/provenance review; do not import a library merely because the prototype used it.
- The GPL Rust service/core remains a **separate development/test host**. It is not embedded, linked, or copied into the iOS app. The native client implements a wire protocol, not a second copy of its gameplay simulation.
- A production shared prediction core must have verified permissive licensing/provenance (or be replaced with an appropriately licensed implementation). Do not silently link the current GPL core into the app or assume that changing a manifest clears its history.
- No proprietary Pico Park art, levels, names, or code is used.
- Before shipping: resolve the production core/server licensing decision, authentication/WSS/origin policy, input scheduling/prediction, long-lived recovery, rendering/assets/audio/accessibility polish, and real-device performance/network tests.

SpriteKit is the current native renderer, **not an irreversible library commitment**. A custom Metal or shared GPU backend should replace it only when profiling or actual rendering requirements justify that cost.
