# Campaign catalogue

All 24 stages have death-free, input-only completion witnesses in `engine/crates/sim-core/src/campaign/tests.rs`: two players for stages 1–22, three for 23, four for 24. Four-player deterministic collision-safety replays cover the full catalogue. These are simulation checks, not a substitute for human difficulty/UX balancing.

| Stage | Name | Lesson / combination |
| --- | --- | --- |
| 1 | A little lift | Stack on a teammate for the shared key |
| 2 | One key, two paths | Retrieve a high key and regroup past the barrier |
| 3 | Wait for me | Terrace-assisted stacking and everyone-at-exit coordination |
| 4 | Hold the welcome | Pressure pad and a forgiving release-grace gate |
| 5 | A breath between beats | Cross a periodic timed gate |
| 6 | Through the underside | Ascend one-way ledges |
| 7 | The patient ferry | Board a moving platform over a hazard |
| 8 | Against the grain | Traverse an opposing conveyor |
| 9 | Room to brake | Brake/countersteer on ice before danger |
| 10 | Borrowed bounce | Use a spring for a high key |
| 11 | Mind the seams | Clear several hazard strips |
| 12 | A place to return | Activate a team checkpoint before hazards |
| 13 | Not for standing | Cross a crumbling bridge before it disappears |
| 14 | Lighter together | Reach a floating key in low gravity |
| 15 | Lean into it | Compensate for wind before a hazard |
| 16 | Relay at dusk | Pressure gate, checkpoint, timed gate, hazard |
| 17 | Cold machinery | Ice, opposing belt, checkpoint, ferry |
| 18 | Updraft workshop | Spring, low gravity, one-way ledge, wind |
| 19 | A bridge on loan | Checkpoint, temporary bridge, pressure and timed gates |
| 20 | The long way home | Pressure relay, ice, ferry, checkpoints, timing, shared lift finale |

## Expedition chapter

The original twenty lessons remain intact. These longer rooms add a substantial vertical climb and a long horizontal quarry crossing:

| Stage | Name | Size (tiles) | Minimum players |
| --- | --- | --- | --- |
| 21 | The shared rope | 64 × 32 | 2 |
| 22 | Quarry crossing | 72 × 36 | 2 |
| 23 | Three on the line | 80 × 40 | 3 |
| 24 | Four to the summit | 88 × 44 | 4 |

Regroup on distinct pads at the rope base. The required number of people must be on the rope to tension it; **hold Jump to climb**, release to descend. This is a cooperative climbing rope, not a swinging/grappling or simulated free-body tether. Fetch the elevated shared key before crossing the combined key/pressure wall. A far-side checkpoint prevents repeating the climb after a quarry accident. Falling rocks have a one-second warning at their landing spots, accelerate down a fixed shaft, then reset. Wait and cross together during a safe window. All retained participants still have to enter the exit, even if more than the minimum joined.

Stages 23/24 deliberately cannot be finished with two identities; join more players to the same room. Their review recordings use the declared team sizes. Review stage 21 directly with `/?solution=21`; review does not unlock or change a live room.

The intended curve introduces tools and then combines them. Strictly increasing measured difficulty is not claimed until human playtests validate the curve. The authoritative catalogue is available at the development host's `/campaign` endpoint.

## How to play

Start the host with `cd engine && cargo run --locked -p multiplayer-server`, run `native/CoopClient.xcodeproj`, and join the same room from two iOS/browser clients. The lowest connected player ID controls **Restart**. See [exit and automatic progression rules](../engine/README.md#exit-review-and-network-diagnostics). A new room starts a new campaign.

An empty room keeps its current stage in a bounded, one-hour, process-local cache. This is not a durable save, an account, or replicated host failover.
