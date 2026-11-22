//! Bounded campaign mechanisms. Coordinates and timers are simulation units/ticks.
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MechanicKind {
    Plate,
    /// Cooperative pad: active only when at least `travel` players (min 2)
    /// stand on it at once. Never solid; drives the same pressure gates a
    /// plate does, so one player cannot open a two-player door.
    TeamPad,
    PressureGate,
    TimedGate,
    OneWay,
    MovingPlatform,
    Conveyor,
    Ice,
    Spring,
    Hazard,
    Checkpoint,
    Crumble,
    LowGravity,
    Wind,
    Rope,
    FallingRock,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mechanic {
    pub id: u32,
    pub kind: MechanicKind,
    pub rect: Rect,
    /// Signed horizontal shuttle distance, belt speed or wind speed; for a
    /// team pad, the players required on it at once: `travel <= 1` means 2
    /// (a threshold of one would just be a plate), clamped to `MAX_PLAYERS`.
    pub travel: i32,
    /// Full shuttle/gate cycle; crumble lifetime; pressure release grace. 0 uses defaults.
    pub period: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanicState {
    pub id: u32,
    pub rect: Rect,
    /// Gates: solid/closed; crumble: present; plate/team pad: occupied;
    /// other zones: enabled.
    pub active: bool,
    pub timer: u32,
}

impl Level {
    /// Reject malformed/unbounded content before performing coordinate arithmetic.
    /// This validates geometry bounds, not puzzle solvability.
    pub fn valid(&self) -> bool {
        let rect_ok = |r: Rect| {
            r.x >= 0
                && r.y >= 0
                && r.w > 0
                && r.h > 0
                && i64::from(r.x) + i64::from(r.w) <= i64::from(self.width)
                && i64::from(r.y) + i64::from(r.h) <= i64::from(self.height)
        };
        (16 * UNIT..=256 * UNIT).contains(&self.width)
            && (14 * UNIT..=128 * UNIT).contains(&self.height)
            && (self.tether_length == 0 || (2 * UNIT..=12 * UNIT).contains(&self.tether_length))
            && self.number <= campaign::LEVEL_COUNT
            && (2..=4).contains(&self.required_players)
            && (PLAYER_HEIGHT..=self.height).contains(&self.spawn_y)
            && self.title.len() <= 128
            && self.hint.len() <= 1024
            && self.solids.len() <= 128
            && self.mechanics.len() <= 64
            && self.solids.iter().all(|&r| rect_ok(r))
            && [self.gate, self.key, self.exit].into_iter().all(rect_ok)
            && self.mechanics.iter().enumerate().all(|(i, m)| {
                rect_ok(m.rect)
                    && m.period <= 3600
                    && (if m.kind == MechanicKind::FallingRock {
                        m.travel >= 0
                            && i64::from(m.rect.y) + i64::from(m.rect.h) + i64::from(m.travel)
                                <= i64::from(self.height)
                    } else {
                        m.travel.unsigned_abs() <= (8 * UNIT) as u32
                    })
                    && !self.mechanics[..i].iter().any(|other| other.id == m.id)
                    && (m.kind != MechanicKind::MovingPlatform || {
                        let end = i64::from(m.rect.x) + i64::from(m.travel);
                        end >= 0 && end + i64::from(m.rect.w) <= i64::from(self.width)
                    })
            })
    }
}

impl World {
    /// Create a restartable level world. To preserve network time, caller sets both
    /// `tick` and `level_start_tick` to its previous tick before the first step.
    pub fn for_level(level: &Level) -> Self {
        let mut world = Self::new();
        if level.valid() {
            world.initialize_objects(level);
        }
        world
    }

    pub(crate) fn initialize_objects(&mut self, level: &Level) {
        self.objects = level
            .mechanics
            .iter()
            .map(|m| MechanicState {
                id: m.id,
                rect: m.rect,
                active: !matches!(
                    m.kind,
                    MechanicKind::Plate
                        | MechanicKind::TeamPad
                        | MechanicKind::Rope
                        | MechanicKind::FallingRock
                ),
                timer: 0,
            })
            .collect();
    }

    pub(crate) fn ensure_objects(&mut self, level: &Level) {
        if self.objects.len() != level.mechanics.len()
            || self
                .objects
                .iter()
                .zip(&level.mechanics)
                .any(|(object, mechanic)| object.id != mechanic.id)
        {
            self.initialize_objects(level);
        }
    }

    pub(crate) fn blocking_objects(&self, level: &Level) -> impl Iterator<Item = Rect> + '_ {
        // Kinds are static level data; object order is always the level's order.
        let kinds: Vec<_> = level.mechanics.iter().map(|m| (m.id, m.kind)).collect();
        self.objects
            .iter()
            .filter(move |o| {
                o.active
                    && kinds.iter().any(|&(id, k)| {
                        id == o.id
                            && matches!(
                                k,
                                MechanicKind::PressureGate
                                    | MechanicKind::TimedGate
                                    | MechanicKind::MovingPlatform
                                    | MechanicKind::Crumble
                            )
                    })
            })
            .map(|o| o.rect)
    }

    pub(crate) fn update_mechanics(&mut self, level: &Level) {
        self.ensure_objects(level);
        let occupied = |r: Rect| self.players.iter().any(|p| p.rect().overlaps(r));
        let standing = |r: Rect| {
            self.players
                .iter()
                .any(|p| p.x < r.x + r.w && p.x + PLAYER_WIDTH > r.x && p.y + PLAYER_HEIGHT == r.y)
        };
        // Distinct bodies, never the same player twice; players are ID-sorted.
        // A body resting exactly on the pad's top edge does not overlap it, so
        // the same predicate backs both counting and the plate check.
        let on_pad = |p: &Player, r: Rect| {
            p.rect().overlaps(r)
                || (p.x < r.x + r.w && p.x + PLAYER_WIDTH > r.x && p.y + PLAYER_HEIGHT == r.y)
        };
        let count_in = |r: Rect| self.players.iter().filter(|p| on_pad(p, r)).count();
        // A team pad needs its own head count, so a two-player pad cannot be
        // held open by one player even on a four-player stage.
        let pad_active = |m: &Mechanic| match m.kind {
            MechanicKind::TeamPad => {
                let need = m.travel.clamp(2, MAX_PLAYERS as i32) as usize;
                let bodies = count_in(m.rect).max(usize::from(standing(m.rect)));
                bodies >= need
            }
            _ => occupied(m.rect) || standing(m.rect),
        };
        let plates: Vec<_> = level
            .mechanics
            .iter()
            .filter(|m| matches!(m.kind, MechanicKind::Plate | MechanicKind::TeamPad))
            .collect();
        let pressed = !plates.is_empty() && plates.iter().all(|m| pad_active(m));
        let phase = self.tick.saturating_sub(self.level_start_tick);
        for (m, o) in level.mechanics.iter().zip(&mut self.objects) {
            match m.kind {
                MechanicKind::Rope => {
                    o.active = self
                        .players
                        .iter()
                        .filter(|p| p.rect().overlaps(m.rect))
                        .count()
                        >= usize::from(level.required_players);
                }
                MechanicKind::FallingRock => {
                    let period = m.period.clamp(180, 3600);
                    o.timer = (phase % u64::from(period)) as u32;
                    let falling = period - 90;
                    let elapsed = o.timer.saturating_sub(60).min(falling);
                    o.active = o.timer >= 60 && o.timer < period - 30;
                    o.rect.y = m.rect.y
                        + (i64::from(m.travel) * i64::from(elapsed).pow(2)
                            / i64::from(falling).pow(2)) as i32;
                }
                MechanicKind::Plate => o.active = occupied(m.rect) || standing(m.rect),
                MechanicKind::TeamPad => o.active = pad_active(m),
                MechanicKind::PressureGate => {
                    if pressed {
                        o.timer = m.period.max(180);
                    } else {
                        o.timer = o.timer.saturating_sub(1);
                    }
                    o.active = o.timer == 0 && !occupied(o.rect);
                }
                MechanicKind::TimedGate => {
                    let period = m.period.clamp(60, 3600);
                    o.timer = (phase % u64::from(period)) as u32;
                    o.active = o.timer < period / 2 && !occupied(o.rect);
                }
                MechanicKind::Crumble => {
                    let life = m.period.clamp(20, 240);
                    if o.timer > 0 {
                        o.timer = o.timer.saturating_add(1);
                    } else if standing(o.rect) {
                        o.timer = 1;
                    }
                    if o.timer >= life {
                        o.active = false;
                    }
                    if o.timer >= life + 180 && !occupied(o.rect) {
                        o.timer = 0;
                        o.active = true;
                    }
                }
                _ => {}
            }
        }
        // Horizontal only. Reject a tick's motion rather than push/crush a player.
        // Riders are translated as a group only if all destination rectangles are free.
        for (index, m) in level.mechanics.iter().enumerate() {
            if m.kind != MechanicKind::MovingPlatform {
                continue;
            }
            let period = m.period.clamp(120, 3600);
            let o = &self.objects[index];
            let timer = (o.timer + 1) % period;
            let half = period / 2;
            let triangle = if timer <= half { timer } else { period - timer };
            let target =
                m.rect.x + (i64::from(m.travel) * i64::from(triangle) / i64::from(half)) as i32;
            let dx = (target - o.rect.x).clamp(-96, 96);
            let proposed = Rect {
                x: o.rect.x + dx,
                ..o.rect
            };
            let riders: Vec<_> = self
                .players
                .iter()
                .filter(|p| {
                    p.x < o.rect.x + o.rect.w
                        && p.x + PLAYER_WIDTH > o.rect.x
                        && p.y + PLAYER_HEIGHT == o.rect.y
                })
                .map(|p| p.id)
                .collect();
            let blockers: Vec<_> = level
                .solids
                .iter()
                .copied()
                .chain((!self.key_collected).then_some(level.gate))
                .chain(
                    self.objects
                        .iter()
                        .enumerate()
                        .filter(|(j, s)| {
                            *j != index
                                && s.active
                                && matches!(
                                    level.mechanics[*j].kind,
                                    MechanicKind::MovingPlatform
                                        | MechanicKind::PressureGate
                                        | MechanicKind::TimedGate
                                        | MechanicKind::Crumble
                                )
                        })
                        .map(|(_, s)| s.rect),
                )
                .collect();
            let safe = !blockers.iter().any(|r| proposed.overlaps(*r))
                && self.players.iter().all(|p| {
                    if riders.contains(&p.id) {
                        let dest = Rect {
                            x: p.x + dx,
                            ..p.rect()
                        };
                        dest.x >= 0
                            && dest.x + dest.w <= level.width
                            && !blockers.iter().any(|r| dest.overlaps(*r))
                            && !self
                                .players
                                .iter()
                                .any(|q| !riders.contains(&q.id) && dest.overlaps(q.rect()))
                    } else {
                        !proposed.overlaps(p.rect())
                    }
                });
            if safe {
                self.objects[index].rect = proposed;
                self.objects[index].timer = timer;
                for p in &mut self.players {
                    if riders.contains(&p.id) {
                        p.x += dx;
                    }
                }
            }
        }
    }

    /// Atomic team failure: preserve the shared key and last checkpoint, reset
    /// mechanisms, and respawn everyone together. A blocked checkpoint falls back.
    pub(crate) fn respawn_team(&mut self, level: &Level) {
        self.deaths = self.deaths.saturating_add(1);
        self.regroup_team(level);
    }

    pub(crate) fn regroup_team(&mut self, level: &Level) {
        let ids: Vec<_> = self.players.iter().map(|p| p.id).collect();
        self.players.clear();
        self.initialize_objects(level);
        for id in &ids {
            if !self.add_player_in(level, *id) {
                self.players.clear();
                self.checkpoint = None;
                for id in &ids {
                    self.add_player_in(level, *id);
                }
                return;
            }
        }
    }
}

impl World {
    /// True when the team can no longer finish: a team pad's gate is shut and
    /// fewer than the needed number of players remain on the pad's side of it,
    /// so the pad can never be pressed again and the key is out of reach.
    ///
    /// This is the "split team" failure of a pad room. Without it the room is
    /// permanently lost — there is no hazard to respawn the team and the pad
    /// cannot be re-pressed from the wrong side.
    pub(crate) fn unwinnable_split(&self, level: &Level) -> bool {
        if self.key_collected || self.completed {
            return false;
        }
        level.mechanics.iter().any(|pad| {
            if pad.kind != MechanicKind::TeamPad {
                return false;
            }
            let need = pad.travel.clamp(2, MAX_PLAYERS as i32) as usize;
            // The gate this pad drives is the nearest pressure gate to its right.
            let Some(gate) = level
                .mechanics
                .iter()
                .filter(|m| m.kind == MechanicKind::PressureGate && m.rect.x > pad.rect.x)
                .min_by_key(|m| m.rect.x)
            else {
                return false;
            };
            // A gate's `active` flag means solid/closed, so the door is only
            // passable when it is false (including during its release grace).
            let gate_closed = self
                .objects
                .iter()
                .zip(&level.mechanics)
                .find(|(_, m)| m.id == gate.id)
                .is_some_and(|(o, _)| o.active && o.timer == 0);
            if !gate_closed {
                return false;
            }
            // Only a door that spans the full traversable height is a real
            // barrier: a short gate can be jumped, and a player mid-air above it
            // would be miscounted as stranded. Requiring the top to sit at least
            // four tiles above the spawn floor keeps a jumpable gate from ever
            // triggering a rescue.
            if gate.rect.y > level.spawn_y - 4 * UNIT {
                return false;
            }
            // Count bodies on each side of the shut door. The run is only lost
            // when the team is genuinely SPLIT: someone is stranded past the
            // door and too few bodies remain behind to press the pad again.
            // A team that all crossed can still reach the key, and a team that
            // all stayed has simply not opened the door yet.
            let behind = self
                .players
                .iter()
                .filter(|p| p.x + PLAYER_WIDTH <= gate.rect.x)
                .count();
            let ahead = self.players.len() - behind;
            // A split needs bodies on BOTH sides. Everyone behind means the door
            // simply has not opened yet; everyone ahead means the team crossed
            // together and can still reach the key.
            if ahead == 0 || behind == 0 {
                return false;
            }
            behind < need
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One team pad on the starting floor; `travel` is the crowd it demands.
    fn team_pad_level(travel: i32) -> Level {
        let mut level = Level {
            number: 28,
            ..Level::default()
        };
        level.mechanics.push(Mechanic {
            id: 1,
            kind: MechanicKind::TeamPad,
            rect: Rect {
                x: 4 * UNIT,
                y: 12 * UNIT - UNIT / 8,
                w: 4 * UNIT,
                h: UNIT / 8,
            },
            travel,
            period: 0,
        });
        level
    }

    fn pad_active(world: &World) -> bool {
        world.objects[0].active
    }

    /// Park a spawned body on the pad without walking-simulation noise.
    fn stand_on_pad(world: &mut World, index: usize, x: i32) {
        let player = &mut world.players[index];
        player.x = x;
        player.y = 12 * UNIT - PLAYER_HEIGHT;
        player.vx = 0;
        player.vy = 0;
        player.grounded = true;
    }

    #[test]
    fn team_pad_needs_two_players() {
        let level = team_pad_level(2);
        let mut world = World::for_level(&level);
        assert!(world.add_player_in(&level, 1));
        stand_on_pad(&mut world, 0, 4 * UNIT);
        world.step(&level, &[]);
        assert!(!pad_active(&world), "one player must not press it");

        assert!(world.add_player_in(&level, 2));
        stand_on_pad(&mut world, 1, 4 * UNIT + PLAYER_WIDTH);
        world.step(&level, &[]);
        assert!(pad_active(&world), "two players must press it");
    }

    #[test]
    fn team_pad_threshold_uses_travel() {
        // travel = 3, so two bodies fall short even though the stage's team
        // size is two; only the third body tips the pad over.
        let level = team_pad_level(3);
        let mut world = World::for_level(&level);
        for id in 1..=3 {
            assert!(world.add_player_in(&level, id));
        }
        stand_on_pad(&mut world, 0, 4 * UNIT);
        stand_on_pad(&mut world, 1, 4 * UNIT + PLAYER_WIDTH);
        stand_on_pad(&mut world, 2, 20 * UNIT);
        world.step(&level, &[]);
        assert!(!pad_active(&world), "two of three must not press it");

        stand_on_pad(&mut world, 2, 4 * UNIT + 2 * PLAYER_WIDTH);
        world.step(&level, &[]);
        assert!(pad_active(&world), "three bodies must press it");
    }

    #[test]
    fn team_pad_ignores_duplicate_bodies() {
        // One player over many ticks: a body counted twice would reach the
        // two-player threshold on its own.
        let level = team_pad_level(2);
        let mut world = World::for_level(&level);
        assert!(world.add_player_in(&level, 1));
        stand_on_pad(&mut world, 0, 4 * UNIT);
        for tick in 0..60 {
            world.step(&level, &[]);
            assert!(!pad_active(&world), "one body counts once (tick {tick})");
        }
    }
}
