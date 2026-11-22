# Multiplayer foundation

An independent successor to the legacy Dino prototype. **Development milestone, not a production engine or a finished iOS game.**

## Run

```sh
cd engine
cargo run --locked -p multiplayer-server
```

Open <http://127.0.0.1:3000> in browser tabs, or join from the [native iOS client](../native/README.md), using the same room code. Move with A/D or arrows; jump with space/W/up. The campaign has **28 original stages**: twenty lessons, a [larger expedition chapter](../docs/CAMPAIGN_LEVELS.md#expedition-chapter) with cooperative rope climbing, falling rocks and explicit 2/3/4-player requirements, and a [platform chapter](../docs/CAMPAIGN_LEVELS.md#platform-chapter) of long vertical courses tied together by a player-to-player tether. All stages have correctly staffed, input-only completion witnesses. Stand on teammates, collect the shared key, and bring everyone to the exit. The room controller can Restart; progression is automatic after team completion. Content is an MVP candidate: human difficulty and touch-feel review are not replaced by completion tests.

### Exit, review and network diagnostics

Entering the unlocked exit commits a player to waiting for the team. Arrived players no longer body-block teammates, even if the first stops at the very lip of the doorway. Body collision/stacking elsewhere is unchanged. Characters shrink/fade into the door (instant disappearance with reduced motion), and reappear if the team respawns. The HUD shows arrivals. After completion the authority waits **900 ms** then advances atomically, preserving identities/ticks and requiring new-revision neutral input baselines. The celebration timer resets while a retained participant is disconnected. The finale stays completed, never wraps to stage 1. Manual Next remains in protocol v3 for compatibility/testing but is no longer exposed by the clients.

Hazards are upright triangular spike teeth in both renderers and the authoritative integer collision test, not invisible bounding rectangles. Empty upper corners/valleys are safe; positive-area tooth/body intersection respawns the team. Teeth count is `clamp(width / 350, 1, 24)` using integer division. Translucent low-gravity/wind/checkpoint zones cannot occlude objectives; the elevated key is rendered above mechanic objects. Native offscreen key indicators point toward objectives above/beyond the camera view.

The browser's **Solution review** panel plays all 28 stages with pause, speed and timeline scrubbing. `/?solution=25` opens a particular stage. Review leaves the live room and sends no gameplay traffic. These are death-free **input-only simulation witnesses**, not mathematically optimal solutions or evidence of real-network performance. Live browser play uses a following camera and offscreen teammate/key indicators; review retains a full-room overview. The renderer consumes recorded authority, not invented animation or injected player coordinates. Mechanic state that never changes during a run is stored once as `constant_objects` instead of in every frame; the viewer fills it in on load. Regenerate recordings after rule/catalog changes:

```sh
COOP_EXPORT_SOLUTIONS="$PWD/web/solutions" cargo test --locked -p sim-core input_only_completion
node --test web/*.test.mjs
```

The test recorder advances an independent replay with the same ordinary inputs and checks every state hash; only completed zero-death runs are exported. Generated recordings are embedded into the server, bounded to the 28 bundled levels and covered by browser contract tests.

Both clients expose **Network statistics**: cumulative sent/received application bytes, session-average KiB/s, applied-input acknowledgment latency and snapshot-gap jitter. Counters never retain content, credentials or addresses. Byte counts **exclude** WebSocket/TLS/TCP/IP framing and retransmissions; applied ack includes scheduling and is **not pure RTT/ping**. These are local diagnostics, not uploaded analytics. Statistics reset their pending ack/gap state on reconnect while keeping session totals.

Clients still sample controls at approximately 60 Hz but send only changed held state, new jump IDs/revisions, controls, or a 100 ms heartbeat. This reduces steady unchanged-input messages from about 60/s to about 9–10/s while preserving the 250 ms input lease and independent acknowledgment timeout. Timer stalls still trigger neutralization/recovery; they are not hidden by the optimization.

### Quick chat

Six server-owned presets — `Follow me`, `Wait here`, `Jump!`, `Ready?`, `Help!`, `Nice!` (indices 0–5) — are the whole vocabulary: clients send only `{"type":"chat","preset":n}` and the server resolves the wording, so there is no free text and no moderation surface. One message per player per 2 s (`CHAT_INTERVAL`); anything faster, out of range or from a disconnected slot is silently ignored, never a disconnect.

Chat rides a per-room broadcast channel drained by the snapshot writer, so it is never part of a snapshot or a patch and costs nothing on the 20 Hz state path. The browser shows a bubble above the speaker for 3.5 s plus a 3-line log; the native client shows a 3 s bubble label (fade skipped under Reduce Motion) and the same 3-line log. Presets, rendering, evidence and scope: [docs/QUICK_CHAT.md](../docs/QUICK_CHAT.md).

### Compact snapshots and finished-screen quiet mode

Hello may opt in with `"patches":true` (browser/native do). The default remains full snapshots for non-opted-in clients. A connection begins with a full snapshot; each successful writer send establishes its next base. Same-revision messages can be `{type:"patch",revision,base_tick,changes}` where nested object keys merge, arrays replace atomically and null **remains a value**, not deletion. `base_tick` must exactly match the last received base; missing/wrong bases cause bounded reconnect. Static level metadata cannot change via a patch. A new revision/level or a patch larger than the full frame sends a full snapshot instead. Crucially, the latest-only mailbox still stores **full** states: patches are generated only at the writer, so skipped mailbox updates cannot drop a required delta. Reconstruction is connection-local and passes the existing native stream validation/64 KiB bounds. No new external dependency or native GPL linkage is introduced.

At the final campaign results screen, clients disable movement and send a neutral keepalive only every **10 seconds**. The server suppresses publications whose only change is the simulation tick; roster/controller/ack/restart changes still publish promptly. The final-screen silence bound is 15 seconds, while the independent **3-second applied-ack deadline** remains enforced for every offered command, including Restart. Restart wakes regular traffic immediately. Active play remains 20 updates/s; inputs, content, level transitions and backpressure safeguards are unchanged. Long browser background suspension may still require rejoining.

The game ruleset is now `coop-campaign-3` to fence clients without tethers and the platform chapter. Refresh browser clients and rebuild native clients after this update. This is still JSON, not a final binary/interest-managed protocol; moving object arrays can remain a substantial cost. Measure actual wire overhead/cellular energy separately from application-payload savings.

### Tethers and presentation smoothing

Stages 25, 27 and 28 set `tether_length` (2–12 tiles). The authority treats the team as a straight slack chain in ascending player-ID order: it iteratively pulls over-stretched pairs back to the limit, biases the correction toward an anchored partner (feet on a solid or one-way ledge), and moves bodies with swept axis clipping so a correction cannot pass through walls. Velocity along the taut axis is removed. If a correction cannot be made valid, the step rolls back to the previous state while preserving consumed jump-held input; if the previous state was also invalid — a late join far from the chain — the team regroups at the checkpoint without charging a death. Tether state is server-side only; clients never send positions.

Remote characters are rendered from a presentation timeline (`engine/web/motion.js`, `native/Sources/MultiplayerKit/MotionTimeline.swift`), never from a guessed authority. It keeps the last 8 authority frames, renders about 100 ms behind the newest arrival (the local player is not delayed), interpolates between frames, and on a late packet extrapolates for at most 100 ms with a 1536-unit distance cap. Output is exponentially smoothed over 35 ms. Any tick rewind, jump over 4096 units, revision change, respawn or roster removal resets the history. This is display-only: collision, goals and acknowledgment still come from snapshots, and the client still sends inputs only.

### Reusable engine boundaries and social/connectivity direction

Generic session identity, transport recovery, resource bounds and diagnostics belong to the multiplayer layer. Campaign geometry/rules and solution controllers belong to the co-op game adapter, not the transport. Current `sim-core`/`net-proto` still include co-op-specific types; this is not yet a completely game-agnostic SDK. Future games should register a versioned game adapter rather than add campaign rules to connection handling. The native statistics utility is original MIT code and does not link the GPL engine.

P2P is **not implemented**. Web browsers can use WebRTC data channels, with signaling and often TURN relay fallback. Before any peer address/ICE exchange, require explicit opt-in explaining IP-address exposure, who the peers are, and the server-relay alternative; trust must be scoped to a session/roster and renewed when peers change. Declining must retain a playable relayed route. Peer consent does not grant simulation authority or bypass validation. Do not silently switch to direct peers. A design review has concluded P2P should be **deferred** rather than built next: the only topology that satisfies the consent rule without exposing peer addresses is a server relay, which adds load to a 2 GB box and buys little over the current authoritative server. If it is ever built, the consent requirements above are its prerequisite.

Quick chat is implemented: preset-only room messages with server-owned wording (`{"type":"chat","preset":0-5}`) on a per-room broadcast channel, one message per player per 2 s, with no free text and no moderation surface. Detail: [docs/QUICK_CHAT.md](../docs/QUICK_CHAT.md). Real text chat still needs server identity binding, length/rate limits, mute/block/report controls, safe text rendering, accessibility, and an explicit retention policy. Voice, global discovery, persisted chat and moderation services remain out of this MVP scope.

The host binds to loopback by default. For trusted LAN testing:

```sh
BIND_ADDR=0.0.0.0:3000 cargo run --locked -p multiplayer-server
```

This is an unauthenticated development service: **do not expose it directly to the Internet**. A room code is a routing identifier, not a password. Hosted clients require HTTPS/WSS, authentication, origin policy, and operational protections before public use.

## Structure

| Path | Responsibility |
| --- | --- |
| `crates/sim-core` | Integer-only fixed-step simulation, collision, stacking, key/gate, cooperative exit, replay checksum |
| `crates/net-proto` | Strict versioned JSON commands, input validation, snapshots, applied sequence acknowledgements |
| `crates/server` | 60 Hz authority, 20 Hz snapshots, isolated rooms, bounded work, WebSocket and diagnostic asset serving |
| `web` | Canvas diagnostic renderer, keyboard/touch controls; never submits positions |

Simulation contains no renderer, transport, platform APIs, random source, or wall clock. The current co-op rules are deliberately concrete; extracting a generic game interface waits until a second game identifies the reusable parts.

## Checks

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
node --check web/client.js
node --test web/client.test.mjs
```

Portability compile checks (install these targets with `rustup target add` first):

```sh
cargo check --locked -p sim-core -p net-proto --target aarch64-apple-ios
cargo check --locked -p sim-core -p net-proto --target aarch64-apple-ios-sim
cargo check --locked -p sim-core -p net-proto --target wasm32-unknown-unknown
```

These do **not** build an iOS app, expose Swift/WASM bindings, or prove on-device determinism. The golden replay runs in host tests; actual ARM-device/WASM replay parity is a later gate.

## Current network contract

1. Connect to `/ws` and send a text `hello` within five seconds:

   ```json
   {"type":"hello","protocol":3,"game":"coop-sandbox","ruleset":"coop-campaign-3","room":"playtest","patches":true}
   ```

   To resume, add optional `"resume_token":"<opaque token from welcome>"`. The room must match. An unknown, expired, or wrong-room token is rejected as `invalid_resume_token`; it never silently creates a player.

2. Receive `welcome` with `type`, `protocol` (3), `ruleset` (`coop-campaign-3`), `player` (u32), `tick_hz` (60), `snapshot_hz` (20), `level`, `revision` (u32), `controller` (player ID or null), `resume_token` (string), `resumed` (boolean). A snapshot follows. Resume returns the original player and **current** campaign stage, even when the room is full or the old socket is still active.
3. Send strictly increasing, nonzero u32 sequence numbers with full held input state and cumulative u32 `jump_id`:

   ```json
   {"type":"input","seq":1,"revision":1,"input":{"axis":1,"jump":false},"jump_id":0}
   ```

   Start `jump_id` at 0, increment once per new discrete press, and send releases with `jump:false` and the same ID. `axis` is -1, 0, or 1. Capture presses at key/pointer-down, not just on a timer. Both `seq` and `jump_id` restart from 0 on every successful welcome (first input has seq 1); they must not wrap. Held jump alone never creates a network edge. Each fresh higher ID sets **one bounded pending edge**; multiple IDs before a tick coalesce, gaps do not synthesize presses, and airborne edges are consumed immediately rather than queued for landing. Stale/duplicate sequence messages cannot change held state, advance jump IDs, or renew the lease. Lower/duplicate jump IDs in fresh-sequence messages cannot create an edge.

4. Full `snapshot` is `{type:"snapshot",world,acks,level,revision,controller,campaign_complete}`. Level adds `number`, `title`, `hint`, `mechanics`, `required_players` (2–4), and `spawn_y` (spawn feet coordinate); world adds `objects`, `deaths`, `checkpoint`, `level_start_tick`. See the compact extension above for opt-in patches. Acks include only players with an **applied** input/control in their current connection generation; a resumed player's old ack is omitted until a new input is applied. `error` remains `{type:"error",code,message}`. No command can set a position, player identity, tick, key, or completion flag. Unknown client fields and older protocol versions are rejected.

5. Controller-only requests are `{type:"restart",seq,revision}` and `{type:"next",seq,revision}`, sharing the input sequence counter. Next requires authoritative completion and stops at level 28. Restart/Next preserve player IDs and world ticks while increasing the revision. Old-revision inputs cannot renew leases or affect the new round. Each player's first accepted input after a transition is a neutral jump-ID baseline; clients pause controls until that sequence is acknowledged. Sequence/jump counters do not reset between stages.

Clients send `{"type":"chat","preset":<0-5>}`; the wording is owned by the server as `net_proto::CHAT_PRESETS`. The server broadcasts `{"type":"chat","player","preset","text"}` on the room channel, never inside a snapshot or patch.

The lowest connected player ID is controller; a disconnected controller hands control to the next connected player. The catalog is available for development inspection at `/campaign`. All geometry, timers, counts, and snapshot size are bounded. Mechanic state is authoritative; renderers do not run a second copy of the rules.

Room codes are 3–16 ASCII letters, digits, or hyphens and are case-sensitive. The server caps the service at 64 rooms, four retained players per room (256 total), **320 upgraded connections** (64 bounded upgrade/replacement headroom), 4096 bytes per message, and 120 incoming frames per peer per second. Outgoing snapshots use a one-value latest watch mailbox, never a FIFO that drops newer authority. Reader and writer run independently with coordinated cancellation; each blocked write/close times out after five seconds. Thirty seconds without a **fresh** input ends a connection; duplicates do not renew this watchdog.

Held input and pending jumps expire on authoritative ticks **250 ms after the last accepted fresh input**, independently of connection lifetime. Disconnect and resume immediately clear pending input and horizontal velocity; neutral input is enforced on the next tick. Vertical gravity/collision physics continues unchanged. The simulation's existing held-edge replay API remains intact; network input uses an explicit-edge entry point. The golden replay was intentionally refreshed for the arrive-and-wait exit behavior. Spawn selection is a bounded checked search of the starting floor, with no unchecked overlapping fallback.

Resume tokens are 32 OS-random bytes encoded as 64 lowercase hex characters, generated with the direct `getrandom` 0.3 dependency (MIT OR Apache-2.0). They are opaque bearer capabilities scoped to a room, not client-supplied player IDs and **not production authentication**. Tokens are not logged. A replacement atomically advances a never-reused connection generation (including across room recreation), stops the old socket, and fences old inputs/disconnect cleanup. Disconnected slots are neutralized immediately and retained for **15 wall-clock seconds**; then the slot/player is removed and an empty room is deleted. A WebSocket close frame with code **1000 permanently leaves** and invalidates the token. Abnormal disconnects, absent close codes, idle timeouts, and code **1001** permit grace/resume; protocol violations and rate failures remove the session permanently. New completion requires every retained participant to be connected (no false win from graced players); a previously completed room stays completed.

The browser's explicit Reconnect retains the token **in memory only**, and only reuses it for the same endpoint and room. It sends application close code **4001** when replacing that socket (browser APIs prohibit sending reserved code 1001), resets sequences on welcome, and never stores the token in the URL or local storage. Code 4001 also permits grace; native transports can send 1001. Reloading loses the token. A failed resume is displayed; the next explicit reconnect can join fresh. This is bounded development recovery, not persistent accounts or matchmaking.

The browser also releases controls after 750 ms without advancing authority, bounds missing applied-input acknowledgments at three seconds and handshake/snapshot silence at five seconds, and retries the captured room with capped backoff. Duplicate ticks cannot renew freshness. Neutral heartbeats continue while a new-revision baseline awaits its ack, so a single expired frame cannot deadlock control synchronization. Protocol/session rejection stops automatic retries and requires an explicit join.

WebSocket read errors are classified through Axum's error source chain: transport I/O/TLS loss, closed connections, and EOF without a close handshake permit grace. Capacity/oversized frames, malformed WebSocket protocol/UTF-8, attack errors, and unknown read failures permanently remove the session (`invalid_websocket`). The direct `tungstenite` dependency uses the same version as Axum's transport for typed classification; error strings and tokens are not logged. Browser input clearing cancels only unsent jump IDs and preserves the last transmitted ID, preventing blur/backgrounding from publishing a previously unsent jump as neutral input.

The server applies the newest held state on its next tick. It does not simulate one tick per message and does not honor a client-supplied clock. Under overload it skips missed timer deadlines instead of catching up in an unbounded burst; simulation consequently slows. These are development policies to measure and revisit, not latency guarantees.

In addition to the twenty pure-simulation completion witnesses, a real two-WebSocket integration test now completes the introductory co-op puzzle using only commands, sends Next, and verifies both peers enter stage 2 with unchanged identities and monotonic authority ticks. It does not inject positions, completion, or keys.

## Deliberate limitations / next gates

- The browser displays snapshots directly: **no prediction, reconciliation, interpolation, or latency compensation yet**. Acknowledgements establish a protocol foundation, not a finished prediction system.
- Empty-room stage progress is retained in a **process-local cache** for up to one hour, capped at 128 saved rooms with oldest-expiry eviction. It stores the current stage/completion, not players, credentials, checkpoint positions, or accounts. After resume grace expires, an explicit fresh join can return to that stage; server restart, TTL, or eviction loses it. This is not durable persistence or replicated failover.
- Discrete presses are protected from a coalesced release, but multiple presses within one tick intentionally coalesce into one edge. A production prediction timeline still needs explicit scheduling.
- JSON is intentionally inspectable for this milestone. The ruleset string is manually versioned, not a cryptographic content hash. Binary codecs/delta compression need measurements first.
- Co-op collision is bounded AABB motion. Horizontal shuttle rider transport is implemented, but slopes, arbitrary moving/crushing platforms, pushing, and general-purpose physics are not. Stable integer transitions and cloning support replay work; **rollback is not implemented**.
- This Rust workspace provides no native iOS shell, Swift bridge, WASM bindings, production GPU renderer, asset pipeline, audio, matchmaking, persistence, monitoring, or public-service hardening. An independent Apple client need not link the GPL Rust engine.
- There is no matchmaking lobby or ready phase. Players join immediately; the room controller owns stage controls, not an authenticated account. Joining a completed room does not silently restart it.
- The host is single-process and room simulation is serialized behind a short-lived lock. Bounded does not mean load-tested; there is no advertised player-count or latency service level.
- Device backgrounding can throttle browser timers; input expires after 250 ms and session recovery is limited to the 15-second disconnect grace. Long suspension needs a fresh join.

See [`../docs/ENGINE_VISION.md`](../docs/ENGINE_VISION.md) for production graphics direction and delivery gates, and [`../docs/BRANCH_AUDIT.md`](../docs/BRANCH_AUDIT.md) for the migration baseline.

## License

The repository contains the GPLv3 license text; see [`../LICENSE`](../LICENSE). The new workspace conservatively declares `GPL-3.0-only`; the legacy license text alone does not establish whether historical contributions were granted under “only” or “or later” terms. Confirm that provenance with the copyright holders before distribution. Before App Store distribution, also review licensing and dependencies with appropriate legal guidance. The root license has not been changed.
