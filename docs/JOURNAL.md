# Development Journal

Append-only log of development sessions. Each entry records what changed, the
evidence that it works, and what comes next. Keep entries short; evidence lives
in the referenced logs and docs.

## 2026-11-22 — Baseline committed (ruleset `coop-campaign-3`)

**State.** 28-stage campaign, player-to-player tethers, platform courses 25–28,
presentation-only motion smoothing on both clients. The previously uncommitted
body of work is now split into 10 logical commits (`aa59169`…`df886f8`) on
`engine/multiplayer-core`, so any part can be reverted independently.

**Known gaps.**

- Solution recordings total 8.8 MB because every frame repeats the level objects.
- `docs/CAMPAIGN_LEVELS.md`, `engine/README.md` and the playtest copy still say
  24 stages.
- Tether rendering has never been seen on a device or in a real browser session.
- The hosted instance still serves the older 24-stage release.

**Next.** Compact the recordings, refresh docs, run the iOS simulator gate,
deploy, then continue gameplay work.

### Same day — recordings compacted, docs refreshed, release deployed

- `perf`: solution recordings now emit `constant_objects` once. Bundle 9.2 MB →
  4.3 MB with identical playback (f3e37d9).
- `docs`: 28-stage catalogue, platform chapter and tether rules documented;
  native and browser headers read `Wire.levelCount` instead of hard-coded 24
  (163b584).
- `deploy`: release `20261122-a9e33b7e8626` installed on Lightsail, hostname
  config reapplied. Verified HTTPS, exact source checksum, all 28 solution
  fixtures, two-player WSS acknowledgments, 401/403 and a private port 3000.
- Gate: Rust workspace, 38 Node, 40 Swift, Clippy, iOS Release build, 5
  simulator UI tests (`ui-gate/run-actlt1gk/Native.xcresult`).

**Next.** Design notes from the co-op research, then new cooperative mechanics
and levels; quick chat is the other open feature.

### Same day — quick chat end to end

- Protocol/server (`30c47e3`): `ClientMessage::Chat` carries a preset index
  only; the server owns the wording (`CHAT_PRESETS`, 6 entries), allows one
  message per player per 2 s and silently ignores anything else. Chat goes out
  over a per-room broadcast channel and never enters a snapshot or a patch, so
  the 20 Hz state path is unchanged.
- Browser: six `[data-chat]` buttons, 3.5 s sprite bubbles, 3-line log;
  40 Node tests pass (`node --test web/*.test.mjs`).
- Native wire (`1c303e4`): preset-only encode, wording validated on decode;
  42 Swift tests in the suite.
- Native UI (`e250c72`): quick-chat row, log and 3 s in-world bubble, committed
  by a delegated builder.
- Unverified: no human playtest of chat, no rate-limit behaviour observed on a
  real network, chat not deployed (the hosted release predates it).

### Same day — delivery fleet changed shape

Subagent throughput was the bottleneck, not the code. The OpenCode free pool
(`free-code`, `free-scribe`, `free-vision`, `free-fallback`) hit its daily cap
mid-session, so two new agents were added on the separate, still-working
CommandCode tier:

- `cc-flint` — `commandcode/stealth/glyph-cluster:free#high`, 40 steps, read-only,
  unlimited. Absorbs audits, extractions, research and drafted diffs.
- `cc-builder` — `commandcode/xiaomi/mimo-v2.6-flash`, 60 steps, the only subagent
  with edit rights. Given one ticket, a named file list, a verification command
  and its own commit.

Probes: `glyph-cluster:free` answered a spawn test; `reflection/beam:free` is dead
("tier has ended"), as is `cat-jam:free` and `laguna-s-2.1-free`. Working agents
were run seven-wide on disjoint files — audit, docs, tests, stage analysis,
mechanic research, chat review, playtest planning — with no conflict, because no
two tickets named the same path and every lane staged by explicit path.

First delegated implementation: `e250c72` (native quick-chat buttons and bubbles)
built, verified and committed by `cc-builder` without orchestrator edits.

### Same day — audits, and a design defect caught before playtest

Nine read-only lanes ran against the same tree while four builders worked
disjoint files. The audits paid for themselves immediately.

**Design defect found and fixed.** Measured from the recorded witnesses, the four
platform courses take 120 s, 291 s, 352 s and 428 s (`web/solutions/25..28.json`
frame counts × stride ÷ 60). Recovery points came only every seventh ledge, so a
late mistake on the 428 s four-player finale could cost minutes of replay. Stride
is now five (`bd7eaee`): checkpoints went 3/6/7/9 → 3/8/10/12 on stages 25–28,
layout untouched, all 39 sim-core tests and the regenerated witnesses still pass.

**Copy drift.** The 24→28 bump left five stale claims: the `hello` example and
welcome prose still said `coop-campaign-2`, the protocol reference said Next
"stops at level 24", and both client headers said 24 stages. Fixed in `eb0032b`
and `25eae92`; the Swift header now interpolates `Wire.levelCount` so that one
cannot drift again.

**Decisions recorded.**

- **P2P: defer.** The consent rule (no peer address or ICE before explicit trust)
  can only be honoured without exposure by a server relay, which loads the 2 GB
  box and buys little over the authoritative server already running. The safety
  rules stay in `engine/README.md` as the prerequisite if it is ever built.
- **Chat: safe to ship.** Preset-only with server-owned wording means no
  attacker-controlled bytes can reach the wire; worst case is 8 broadcast sends
  per second across a full room.
- **Next mechanic: team pad.** No new `World` field, no hash change, no patch
  encoder work — an enum arm, a count check and a glyph.

**Open bug.** Chat frames share the gameplay frame budget, so chat spam could
consume `INPUTS_PER_SECOND` and drop a player's input or the connection. Fix in
flight: chat gets its own budget and is dropped rather than fatal.
