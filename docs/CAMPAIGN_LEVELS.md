# Campaign catalogue

All 32 stages have death-free, input-only completion witnesses in `engine/crates/sim-core/src/campaign/tests.rs`: two players for stages 1–22, 25, 26, 29 and 30; three for 23, 27 and 31; four for 24, 28 and 32. Four-player deterministic collision-safety replays cover the full catalogue. These are simulation checks, not a substitute for human difficulty/UX balancing.

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

## Platform chapter

The last four rooms are long, vertical platform courses rather than single rooms. They use the player-to-player tether introduced in `coop-campaign-3`: a straight, slack chain in ascending player-ID order. A taut tether pulls teammates together, an anchored partner can catch a fall, and corrections never pass through walls.

| Stage | Name | Size (tiles) | Minimum players | Tether |
| --- | --- | --- | --- | --- |
| 25 | Tied at the ridge | 128 × 56 | 2 | 6 tiles |
| 26 | The long switchback | 160 × 64 | 2 | none |
| 27 | Three across the sky | 192 × 80 | 3 | 6 tiles |
| 28 | The high road home | 224 × 96 | 4 | 6 tiles |

Each course climbs a staircase of ledges, turns back above the outbound route on one-way ledges, and puts the key and exit on the upper return. Checkpoints sit every seventh platform. Keep slack for jumps, wait on safe ledges, and let an anchored teammate catch a fall. Stage 26 removes the tether so the same geometry can be read without rope pressure; stage 27 and 28 then add team size on top of the tether.

The tether is a chain constraint, not a simulated rope: it does not wrap, sag, or swing. If a correction would stretch it or push a body through geometry, the step rolls back; if a new identity joins far from the chain, the team regroups at the checkpoint without charging a death.

The intended curve introduces tools and then combines them. Strictly increasing measured difficulty is not claimed until human playtests validate the curve. The authoritative catalogue is available at the development host's `/campaign` endpoint.

## Team pad chapter

Two rooms teach `MechanicKind::TeamPad`, a pad that is pressable only while the required number of players stand on it at once. A rest beat sits between them, and the campaign closes on a four-player finale built from the same pad:

| Stage | Name | Size (tiles) | Minimum players | Lesson |
| --- | --- | --- | --- | --- |
| 29 | Both on the brass | 30 × 14 | 2 | Hold the brass pad together, then split into holder and key-runner |
| 30 | A moment to breathe | 30 × 14 | 2 | Rest beat: one familiar spring, no timing or hazards |
| 31 | Two to hold | 34 × 14 | 3 | Two hold the brass while the third carries the key home |
| 32 | All hands on brass | 20 × 14 | 4 | Finale: three hold a three-body pad, the fourth springs for the key |

The pad gates the door in the middle of the corridor, and the key sits behind that door: the team must keep the pad held down while a teammate walks through, collects the shared key and comes back. One player alone can never finish — stage 29 needs one to hold and one to run, and stage 30 leaves two on the brass while the third carries the key home. Stage 30 drops the pressure entirely — two springs, a shelf above the key and no hazards — so the team catches its breath before the last two rooms. Stage 32 is the finale: the pad needs three bodies, so the fourth player is free to ride a spring for a key six tiles up, and everyone has to meet at the exit.

No pad room can become unwinnable. If the authority finds a pad's gate shut with too few bodies left on the pad's side, it records `rescued_at`, charges one death and regroups the whole team, and both clients show a short "run lost — team was split" message.

## How to play

Start the host with `cd engine && cargo run --locked -p multiplayer-server`, run `native/CoopClient.xcodeproj`, and join the same room from two iOS/browser clients. The lowest connected player ID controls **Restart**. See [exit and automatic progression rules](../engine/README.md#exit-review-and-network-diagnostics). A new room starts a new campaign.

An empty room keeps its current stage in a bounded, one-hour, process-local cache. This is not a durable save, an account, or replicated host failover.
