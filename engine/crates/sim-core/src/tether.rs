//! Authoritative, integer-only slack tethers. No client-position authority.
use crate::*;

impl World {
    fn arrived(&self, level: &Level, player: &Player) -> bool {
        self.key_collected && player.rect().overlaps(level.exit)
    }

    fn rope_solids(&self, level: &Level) -> Vec<Rect> {
        level
            .solids
            .iter()
            .copied()
            .chain((!self.key_collected).then_some(level.gate))
            .chain(self.blocking_objects(level))
            .collect()
    }

    fn anchored(&self, level: &Level, index: usize) -> bool {
        let p = &self.players[index];
        self.rope_solids(level)
            .into_iter()
            .chain(
                level
                    .mechanics
                    .iter()
                    .filter(|m| m.kind == MechanicKind::OneWay)
                    .map(|m| m.rect),
            )
            .any(|r| p.y + PLAYER_HEIGHT == r.y && p.x < r.x + r.w && p.x + PLAYER_WIDTH > r.x)
    }

    // Swept axis clipping rather than teleporting a correction through a wall.
    fn rope_move(&mut self, level: &Level, index: usize, dx: i32, dy: i32) {
        let mut solids = self.rope_solids(level);
        solids.extend(
            self.players
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != index)
                .map(|(_, p)| p.rect()),
        );
        let p = &mut self.players[index];
        let mut x = (p.x + dx).clamp(0, level.width - PLAYER_WIDTH);
        for r in &solids {
            if p.y >= r.y + r.h || p.y + PLAYER_HEIGHT <= r.y {
                continue;
            }
            if dx > 0 && p.x + PLAYER_WIDTH <= r.x && x + PLAYER_WIDTH > r.x {
                x = x.min(r.x - PLAYER_WIDTH);
            }
            if dx < 0 && p.x >= r.x + r.w && x < r.x + r.w {
                x = x.max(r.x + r.w);
            }
        }
        p.x = x;
        let mut y = (p.y + dy).max(0);
        solids.extend(
            level
                .mechanics
                .iter()
                .filter(|m| {
                    m.kind == MechanicKind::OneWay && dy > 0 && p.y + PLAYER_HEIGHT <= m.rect.y
                })
                .map(|m| m.rect),
        );
        for r in &solids {
            if p.x >= r.x + r.w || p.x + PLAYER_WIDTH <= r.x {
                continue;
            }
            if dy > 0 && p.y + PLAYER_HEIGHT <= r.y && y + PLAYER_HEIGHT > r.y {
                y = y.min(r.y - PLAYER_HEIGHT);
            }
            if dy < 0 && p.y >= r.y + r.h && y < r.y + r.h {
                y = y.max(r.y + r.h);
            }
        }
        p.y = y;
        if dy < 0 {
            p.grounded = false;
        }
    }

    fn tether_valid(&self, level: &Level, players: &[Player]) -> bool {
        players.windows(2).all(|pair| {
            if pair.iter().any(|p| self.arrived(level, p)) {
                return true;
            }
            let dx = i64::from(pair[1].x - pair[0].x);
            let dy = i64::from(pair[1].y - pair[0].y);
            dx * dx + dy * dy <= i64::from(level.tether_length + 16).pow(2)
        })
    }

    /// `key_before_tether` is the pre-movement key state: movement may gather the
    /// key before this solve runs, and a rollback must undo that with the players.
    pub(crate) fn solve_tether(
        &mut self,
        level: &Level,
        previous: Vec<Player>,
        key_before_tether: bool,
    ) {
        if !self.tether_valid(level, &previous) {
            // A regroup relocates the team but keeps the tick, so a key touched
            // on this step stays collected (death respawn preserves it too).
            // Only a rollback, which discards the tick, undoes the pickup.
            self.regroup_team(level);
            return;
        }
        for _ in 0..8 {
            for b in 1..self.players.len() {
                let a = b - 1;
                if self.arrived(level, &self.players[a]) || self.arrived(level, &self.players[b]) {
                    continue;
                }
                let dx = i64::from(self.players[b].x - self.players[a].x);
                let dy = i64::from(self.players[b].y - self.players[a].y);
                let squared = dx * dx + dy * dy;
                if squared <= i64::from(level.tether_length).pow(2) {
                    continue;
                }
                let root = (squared as u64).isqrt() as i64;
                let distance = root + i64::from(root * root < squared);
                let excess = distance - i64::from(level.tether_length);
                let anchored_a = self.anchored(level, a);
                let anchored_b = self.anchored(level, b);
                let share_a = if anchored_a && !anchored_b {
                    0
                } else if anchored_b && !anchored_a {
                    2
                } else {
                    1
                };
                let share_b = 2 - share_a;
                self.rope_move(
                    level,
                    a,
                    (dx * excess * share_a / distance / 2) as i32,
                    (dy * excess * share_a / distance / 2) as i32,
                );
                self.rope_move(
                    level,
                    b,
                    (-dx * excess * share_b / distance / 2) as i32,
                    (-dy * excess * share_b / distance / 2) as i32,
                );
                let radial = ((i64::from(self.players[b].vx - self.players[a].vx) * dx
                    + i64::from(self.players[b].vy - self.players[a].vy) * dy)
                    / distance)
                    .max(0);
                for (index, sign, share) in [(a, 1, share_a), (b, -1, share_b)] {
                    self.players[index].vx += (sign * dx * radial * share / distance / 2) as i32;
                    self.players[index].vy += (sign * dy * radial * share / distance / 2) as i32;
                }
            }
        }
        if !self.tether_valid(level, &self.players) {
            if self.tether_valid(level, &previous) {
                // A blocked rope cannot phase bodies through walls or stretch.
                // Preserve consumed jump-held state: rollback must not queue jumps.
                for (old, current) in previous.into_iter().zip(&mut self.players) {
                    let held = current.jump_held;
                    *current = old;
                    current.jump_held = held;
                }
                // Movement gathered the key before this solve; undo it with the players.
                self.key_collected = key_before_tether;
            } else {
                // Unreachable in practice: line 103 already returned for an
                // invalid pre-move chain, so this branch's guard always holds.
                self.regroup_team(level);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn taut_teammates_pull_each_other_without_crossing_walls() {
        let level = Level {
            tether_length: 3 * UNIT,
            ..Level::default()
        };
        let mut world = World::for_level(&level);
        world.add_player_in(&level, 1);
        world.add_player_in(&level, 2);
        for _ in 0..600 {
            world.step(
                &level,
                &[
                    (
                        1,
                        Input {
                            axis: -1,
                            jump: false,
                        },
                    ),
                    (
                        2,
                        Input {
                            axis: 1,
                            jump: false,
                        },
                    ),
                ],
            );
            assert!(world.tether_valid(&level, &world.players));
            for p in &world.players {
                assert!(!p.rect().overlaps(level.gate));
            }
            assert!(!world.players[0].rect().overlaps(world.players[1].rect()));
        }
        assert!(world.players[0].x > 0, "the tether must resist separation");
    }
    #[test]
    fn regroup_relocates_but_intentionally_keeps_a_key_gathered_that_step() {
        let level = Level {
            tether_length: 4 * UNIT,
            // Floor level, one leftward stride from player 1's staging spot and
            // nowhere near the spawn columns the regroup lands on.
            key: Rect {
                x: 17_856,
                y: 12 * UNIT - PLAYER_HEIGHT,
                w: UNIT / 2,
                h: UNIT / 2,
            },
            ..Level::default()
        };
        assert!(level.valid());
        let mut world = World::for_level(&level);
        assert!(world.add_player_in(&level, 1));
        assert!(world.add_player_in(&level, 2));
        // The pair walks far from spawn together: still a valid chain...
        world.players[0].x = 18 * UNIT;
        world.players[1].x = 18 * UNIT + UNIT;
        assert!(world.tether_valid(&level, &world.players));
        // ...until a late join spawns at the columns, far from that chain.
        assert!(world.add_player_in(&level, 3));
        assert!(
            !world.tether_valid(&level, &world.players),
            "the pre-move chain must be tether-invalid for the early regroup path"
        );
        assert!(!world.key_collected);
        let resting = world.players[0].rect();
        assert!(!resting.overlaps(level.key), "standing must not gather it");
        let walked = Rect {
            x: resting.x - 96,
            ..resting
        };
        assert!(walked.overlaps(level.key), "one stride must gather it");
        world.step(
            &level,
            &[(
                1,
                Input {
                    axis: -1,
                    jump: false,
                },
            )],
        );
        // The step stands: the team is relocated, not rolled back...
        assert_eq!(world.deaths, 0, "a tether regroup charges no death");
        assert!(
            world.players.iter().all(|p| p.x < 8 * UNIT),
            "the regroup must relocate everyone to the spawn columns"
        );
        assert!(world.tether_valid(&level, &world.players));
        assert!(
            world.players.iter().all(|p| !p.rect().overlaps(level.key)),
            "no regrouped body may be re-touching the key"
        );
        // ...so the key the player physically touched this tick stays gathered.
        assert!(
            world.key_collected,
            "intentional: a regroup keeps a key gathered on the regrouping step"
        );
    }
}
