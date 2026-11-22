# Human playtest script

For 2–4 people on one sitting. The order is deliberate: the riskiest unknowns
(tether readability, motion smoothing under load, long-course recovery) come
first, so the session still yields useful answers if it is cut short.

Hosted: <https://mobile.userorientedsystems.com/> (`wss://…/ws`). Refresh browser
tabs and rebuild native clients after every deploy. The playtest access code lives
outside the repository; see `deploy/lightsail/README.md`.

At the end of the session, press **Download playtest log** (under "Network
statistics" on the playtest page) and keep the resulting `coop-playtest-*.json`
file alongside these notes — `docs/PLAYTEST_LOG.md` explains what it records and
how to read it.

## Block 1 — tether and motion smoothing under load (≈15 min)

- **Do.** Two players, stage 25 (*Tied at the ridge*, 6-tile tether). Mid-climb,
  degrade one device for 60 s: carry it 1–2 rooms from the router, or start a
  large background download/stream on it. Then deliberately fall once while the
  partner stands on a solid ledge.
- **Watch.** Is the tether line drawn and readable? Does the anchored partner
  actually catch the fall? Does the remote player move smoothly? Do inputs still
  register?
- **Broken.** Tether invisible, corrections pass through geometry, desync or a
  reconnect loop, inputs ignored for more than 3 s.
- **Uncomfortable.** Sub-tile rubber-banding, a brief catch-up after the network
  recovers, or a tether that feels punitive but is recoverable.
- **Record.** Largest teleport (tiles), recovery time (s), tether visible (Y/N),
  any pass-through (Y/N).

**Timeline signal.** Smooth stepwise catch-up is correct behaviour. Snaps larger
than one tile, freeze-then-jump, or oscillation near the anchor mean the
presentation buffer needs retuning, not that the simulation is wrong.

## Block 2 — team-size gates (≈10 min)

- **Do.** Attempt stage 23 with two players, then add a third. Attempt stage 24
  with three, then add a fourth. Spot-check the minimums on 27 and 28. Confirm
  that extra retained players can still reach the exit.
- **Watch.** Whether join feedback explains the requirement, and whether
  completion actually respects the declared minimum.
- **Broken.** A stage completes below its minimum, or cannot be completed at it.
- **Record.** Stage, players present, completed (Y/N), seconds to understand the
  gate.

## Block 3 — long platform courses (≈20 min)

- **Do.** Play 25→28 at required team size (25/26: 2, 27: 3, 28: 4). On each,
  fall deliberately after a checkpoint. Compare stage 26 (no tether) against the
  tethered courses.
- **Watch.** Camera framing, checkpoint regroup, tether slack and catches on long
  climbs, and whether the return route reads as a route.
- **Broken.** Checkpoint restore fails, a tether correction pushes a body through
  a wall, a fall forces a full restart, or the camera loses the active player for
  more than 3 s.
- **Uncomfortable.** More than seven platforms replayed after a fall, camera
  jitter, or a tether that costs more coordination than it returns in fun.
- **Record.** Falls per stage, longest replay (platforms), camera-loss incidents,
  "would replay this stage?" (Y/N).

## Block 4 — network sanity (≈5 min, can run inside any block)

- **Do.** One device on normal Wi-Fi, one degraded. Stand still for 10 s, then
  do a coordinated jump.
- **Watch.** Remote smoothing, snapshot catch-up, input acknowledgement feel.
- **Broken.** Permanent divergence, repeated reconnects, controls dead after the
  network recovers.

## Results table

Copy into `docs/JOURNAL.md` and fill in during play.

| Block | Stage | Team | Device / network | Broken? | Uncomfortable? | Key number | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 tether+net | 25 | 2 | | | | teleport __ tiles; recover __ s | |
| 1 tether+net | 25 | 2 | | | | tether visible Y/N | |
| 2 team gate | 23 | 2→3 | | | | understood __ s | |
| 2 team gate | 24 | 3→4 | | | | understood __ s | |
| 3 course | 26 | 2 | | | | falls __; replay __ platforms | |
| 3 course | 27 | 3 | | | | falls __; replay __ platforms | |
| 3 course | 28 | 4 | | | | falls __; replay __ platforms | |
| 4 net sanity | any | 2 | | | | recover __ s | |
| free play | | | | | | | |
| free play | | | | | | | |

## Three answers that change the next sprint

1. **Is the tether readable and fair under latency?** If not, the next sprint is
   tether feedback and rendering, not new mechanics.
2. **Does motion smoothing hide jitter or invent positions?** If snaps or
   oscillation appear, retune the presentation timeline before adding content.
3. **Do team gates and long courses create coordination or dead time?** If
   minimum-player confusion or long replays dominate, the next sprint is gating
   UX and checkpoint pacing, not more chapter.
