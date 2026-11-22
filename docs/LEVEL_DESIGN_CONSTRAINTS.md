# Level design constraints from the simulation

Derived by reading the step and collision code, then confirmed the hard way: two
team-pad levels were designed wrongly before these rules were written down. Treat
this as the checklist a level must pass before it ships.

## The blocking rule, exactly

Player bodies are `720 × 896` units (one tile is 1024). Players are **solid to
each other**. A player's proposed rectangle is tested against every other
player's current rectangle; horizontal movement into another body is clamped and
stops dead. There is no push, no squeeze and no sliding past.

Two consequences follow immediately:

1. **Two players cannot pass each other in a corridor one body wide.** Whoever is
   in front stays in front. Any corridor that both directions must use, or that a
   player must re-traverse, needs **at least `2 × 720 = 1440` units** clear, or a
   vertical alternative.
2. **Vertical stacking works, horizontal passage does not.** A falling player
   lands on another player's head and is grounded there. Stacking is a legitimate
   route; walking through a teammate is not.

## Resolution order

Movement resolves once per tick, ordered by **lowest on screen first, then lowest
ID**. Earlier movers' new positions are visible to later movers, so the player at
the front of a queue moves before the player behind them. This is why a witness
walking "front-most first" succeeds where ascending-ID order jams.

There is no unwedging mechanic and no wedged-state detector. A level that can
trap a body is a broken level, not a hard level.

## The exit carve-out

A player standing in the exit **after the key is collected** stops being a body:
their velocity is zeroed, they are skipped, and they are removed from everyone
else's obstacle list. Before the key, the exit is not special. This is what lets
a team queue into a narrow exit.

## Respawning is safe

`respawn_team` clears the players, reinitialises objects, and re-adds each player
by ID through `add_player_in`, which searches spawn columns for a rectangle that
overlaps no other player, no solid, no active gate and no blocking object. It
falls back to clearing the checkpoint and retrying from level spawn. A respawn
therefore cannot place a body inside another body.

## Checklist for a new level

- [ ] Every mandatory corridor that must carry more than one player in the same
      direction is at least **1440 units** wide, or has a vertical bypass.
- [ ] Any corridor that must be traversed twice has a **passing pocket** at least
      1440 units wide — otherwise the returning player is trapped behind the one
      who stayed.
- [ ] Pads sit **off the main path**, with room for `travel` bodies side by side
      (`travel × 720` plus spacing) and a way to leave without crossing the team.
- [ ] A pad-controlled door never separates a player from the rest of the team in
      a way that the door's release grace cannot cover. Measured walking speed is
      roughly one tile per 20 ticks; a 900-tick grace covers about 45 tiles.
- [ ] The exit has space for `required_players` bodies at once, or the team can
      arrive and wait without blocking the approach.
- [ ] A death-free, input-only witness actually completes the level. If the
      witness cannot finish it, the geometry is wrong — do not loosen the witness.

## Why the checkpoints matter

Platform courses place a checkpoint every fifth ledge. The completion witness
passes with *any* checkpoint layout, so a witness alone cannot catch sparse
recovery points; stage 28 was a 428-second four-player climb with recovery only
every seventh ledge before this was noticed. Spacing is now pinned by its own
test rather than inferred from a successful run.
