# Co-op design notes

Working notes from a survey of coordination games (Pico Park, Chained Together,
It Takes Two, Overcooked, Bread & Fred, Snipperclips, Keep Talking and Nobody
Explodes). These are design inputs, not requirements; the campaign still needs
human playtests before any curve claim.

## What makes coordination levels work

1. **The goal is collective, not shared.** Pico Park's single key and
   everyone-at-exit rule make the finish line a team state. Our key + exit +
   retained-participant rule already does this; keep it.
2. **Bodies must matter.** Snipperclips and Bread & Fred make the other player a
   tool. Stacking, tether tension and anchored catches are our version of that.
   Mechanics that can be solved alone should be rare.
3. **Separation must be limited, not impossible.** Chained Together's chain makes
   spacing everyone's problem. Our tether should create tension without turning
   every jump into a group failure.
4. **Every player needs a job.** It Takes Two's asymmetry and Overcooked's
   parallel stations avoid one player waiting. A gate that only needs one person
   to stand still is the failure mode to avoid.
5. **Failure must be legible and cheap.** Chained Together's beginner mode
   respawns at the checkpoint; It Takes Two keeps the run alive while one partner
   is down. Our regroup-without-death rule follows this.
6. **Teach with geometry.** Pico Park's themed gimmick sets and It Takes Two's
   visual affordances explain mechanics without text. Level layout, not hints,
   should carry the lesson.
7. **Twist before combining.** The common pattern is teach → combine → twist →
   rest. Our catalogue mostly combines; it needs more single-mechanic rest levels
   between dense ones.

## Rules adopted for new content

- A new mechanic gets one safe level, then one combined level, then a rest level.
- No level should be completable by one player while others idle.
- A death or fall must be recoverable within a few seconds; never replay a long
  climb for a small mistake.
- Signals are visual: distinct pad colours, key placement, camera framing.

## Level ideas that fit the current engine

- **Counterweight key** — key above a one-way ledge; a pad lowers the route.
  One player holds, the others cross, then the tether regroups them.
- **Split crew** — a team-size gate divides upper and lower routes; a pad holds
  the return open while the key carrier crosses.
- **Tethered switchback** — key beyond a gap with a pad-controlled bridge; take
  turns crossing while one anchors on safe ground.

The first two are implementable with existing mechanics; the third needs a
pad-driven bridge, which is the next mechanical addition worth building.
