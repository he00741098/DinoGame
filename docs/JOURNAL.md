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
