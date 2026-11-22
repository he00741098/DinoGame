# Team pad

`MechanicKind::TeamPad` (commit `bc626e3`, `engine/crates/sim-core/src/mechanics.rs`) is a pressure pad that activates only while **at least N players stand on it at once**; one body, however long it waits, never presses it (`team_pad_ignores_duplicate_bodies`).

## Where the threshold lives

`N` is the mechanic's own `travel` field, clamped at runtime to `2..=4` (`m.travel.clamp(2, 4)`). It is **per-pad, not per-level**: the stage's `required_players` (2–4) never enters the count, so a four-player stage can carry a two-player pad that one player alone can never hold open — `team_pad_threshold_survives_a_larger_team` pins two bodies pressing a two-player pad inside a three-player stage.

## Counting rules

- **Distinct bodies**: the count iterates `World.players`, so each player counts once.
- **ID order**: `World.players` is kept sorted by ID (never connection or map order), so the count is taken in the same deterministic order every tick.
- **Overlap or standing**: a rectangle overlap *or* feet resting exactly on the pad's top edge both count (`on_pad`), so a row of edge-standing players is never undercounted (`team_pad_counts_bodies_resting_on_its_edge`).

## What it drives

The same pressure-gate linkage an ordinary plate drives: `pressed` is the AND over every `Plate` and `TeamPad` in the level, and a `PressureGate` refreshes its release-grace timer while `pressed` holds — a team pad can therefore hold a gate open through the grace after the team steps off. It is never solid: absent from `blocking_objects`, and it starts inactive like a plate.

## Snapshot cost — why this mechanic was chosen

A team pad is an ordinary `Mechanic` plus one ordinary `MechanicState` entry in `World.objects`. Explicitly: **no new `World` field, no `state_hash` change** (canonical version 2 still hashes objects generically), **no patch-encoder change** (`snapshot_patch.rs` is kind-agnostic JSON). The cost/benefit audit picked it for exactly this: a new state field would have forced a hash/version bump on every client.

## What it does NOT do

- No per-pad colour coding: every team pad renders one fixed colour, whatever N.
- No partial or progress feedback: state is binary (`active`), never "2 of 3".
- No free-standing logic beyond gate pressure: it never moves a body, opens the key gate or touches the exit; it only reports pressable state to gates.

## Failure and recovery

Two mechanisms keep the pad rooms (stages 29 and 31) from going unwinnable; both live in the room, not in the pad mechanic.

- **Automatic rescue** — `World::unwinnable_split` (`mechanics.rs`), checked every tick by the authority. Fires only when a pad's nearest pressure gate to its right is shut (`active && timer == 0`), the team is genuinely split across it (bodies on **both** sides), too few bodies remain pad-side to re-press the pad (`behind < need`), **and** the gate is a full-height barrier (`gate.rect.y <= level.spawn_y - 4 * UNIT`) — a jumpable gate never triggers it, so a player mid-air above a short door is never miscounted as stranded. On fire it sets `world.rescued_at`, charges one death and regroups the team: headless, automatic, no player action. It returns `false` once the key is collected (or the run is completed), so the rescue never fires after the key.
- **Manual hazard** — `campaign.rs`'s `team_pad_room` authors a `Checkpoint` on the **pad side** of the door (a reset must return the team where the pad can still be pressed) and a `Hazard` strip on the overhead one-way ledge, far side of the door and above the walking line. A stranded player climbs to it deliberately; the touch charges a death and respawns everyone at the pad-side flag, keeping the shared key. This is the route after the key is collected — which is why the hazard exists: the automatic rescue is skipped then.

Neither can soft-lock: before the key the authority rescues on its own, after the key the hazard is a deliberate hand reset.

## Rendering (both clients)

Both draw from the authoritative `world.objects`; neither re-simulates rules.

- **Browser** (`engine/web/client.js`): `team_pad: ['#e87fb5', '👥']` — a pink box (`#394356` while inactive), the standard object caption (`👥 #id ON/off t:…`), plus a bold `×N` above an inactive pad (N = the pad's `travel`, minimum 2; the authored 2–4 range matches the sim clamp).
- **Native** (`native/iOS/GameScene.swift`): teal (`.systemTeal`) at full alpha, no zone ghosting, squashing to `yScale 0.55` when active exactly like a plate; caption `N TO PRESS` idle / `ENGAGED` pressed, legend `◎ Team pad: stand together to press`. Its `padThreshold` clamps `travel` to `2...4`, falling back to the level team size when `travel` carries no count.

## Authoring

`campaign.rs`'s helper is `add(l, kind, rect, travel, period)`; the call shape:

```rust
add(&mut l, TeamPad, rect, travel, 0); // travel = threshold, period unused
```

`valid()` only bounds `|travel| <= 8 * UNIT`, so the runtime clamp enforces `2..=4` (`0`/`1` → `2`, `≥ 4` → `4`): author `2`, `3` or `4`. The campaign's `team_pad_room` (stages 29–30) uses `travel = 2`.
