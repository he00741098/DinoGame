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
