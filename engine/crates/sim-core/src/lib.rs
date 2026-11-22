//! Fixed-tick, integer-only cooperative platformer. No clock, sockets, or renderer.
use serde::{Deserialize, Serialize};

pub mod campaign;
mod mechanics;
mod tether;
pub use mechanics::{Mechanic, MechanicKind, MechanicState};

pub const TICK_HZ: u32 = 60;
pub const UNIT: i32 = 1024;
pub const MAX_PLAYERS: usize = 4;
pub const PLAYER_WIDTH: i32 = 720;
pub const PLAYER_HEIGHT: i32 = 896;
pub type PlayerId = u32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub axis: i8,
    pub jump: bool,
}

impl Input {
    pub fn valid(self) -> bool {
        (-1..=1).contains(&self.axis)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn overlaps(self, other: Self) -> bool {
        i64::from(self.x) < i64::from(other.x) + i64::from(other.w)
            && i64::from(self.x) + i64::from(self.w) > i64::from(other.x)
            && i64::from(self.y) < i64::from(other.y) + i64::from(other.h)
            && i64::from(self.y) + i64::from(self.h) > i64::from(other.y)
    }
}

/// Positive-area intersection with the upright spike teeth used by both renderers.
/// Evaluate the widest horizontal slice inside the body using integer products;
/// the empty upper corners of the old rectangular hazard are safe now.
fn touches_spikes(body: Rect, hazard: Rect) -> bool {
    if hazard.w <= 0 || hazard.h <= 0 || !body.overlaps(hazard) {
        return false;
    }
    let teeth = (hazard.w / 350).clamp(1, 24);
    let width = i64::from(hazard.w);
    let height = i64::from(hazard.h);
    let bottom = (i64::from(body.y) + i64::from(body.h) - i64::from(hazard.y)).min(height);
    let left = (i64::from(body.x) - i64::from(hazard.x)) * 2 * i64::from(teeth) * height;
    let right = (i64::from(body.x) + i64::from(body.w) - i64::from(hazard.x))
        * 2
        * i64::from(teeth)
        * height;
    (0..teeth).any(|i| {
        let tip = i64::from(2 * i + 1) * width * height;
        right > tip - width * bottom && left < tip + width * bottom
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    /// Straight, slack player-to-player chain in sorted identity order. 0 = untied.
    #[serde(default)]
    pub tether_length: i32,
    #[serde(default = "minimum_team")]
    pub required_players: u8,
    #[serde(default = "default_spawn_y")]
    pub spawn_y: i32,
    #[serde(default)]
    pub number: u8,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub hint: String,
    #[serde(default)]
    pub mechanics: Vec<Mechanic>,
    pub width: i32,
    pub height: i32,
    pub solids: Vec<Rect>,
    pub gate: Rect,
    pub key: Rect,
    pub exit: Rect,
}

fn minimum_team() -> u8 {
    2
}
fn default_spawn_y() -> i32 {
    12 * UNIT
}

impl Default for Level {
    fn default() -> Self {
        Self {
            tether_length: 0,
            required_players: 2,
            spawn_y: 12 * UNIT,
            number: 0,
            title: "Shoulder to shoulder".into(),
            hint: "Stand on a friend, release jump, then jump again for the shared key.".into(),
            mechanics: Vec::new(),
            width: 24 * UNIT,
            height: 14 * UNIT,
            solids: vec![Rect {
                x: 0,
                y: 12 * UNIT,
                w: 24 * UNIT,
                h: 2 * UNIT,
            }],
            // Too high to jump onto alone. The key retracts it for the teammate below.
            gate: Rect {
                x: 9 * UNIT,
                y: 9 * UNIT,
                w: 3 * UNIT,
                h: 3 * UNIT,
            },
            key: Rect {
                x: 10 * UNIT,
                y: 8 * UNIT,
                w: UNIT / 2,
                h: UNIT / 2,
            },
            exit: Rect {
                x: 22 * UNIT,
                y: 10 * UNIT,
                w: UNIT,
                h: 2 * UNIT,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub grounded: bool,
    pub jump_held: bool,
}

impl Player {
    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: PLAYER_WIDTH,
            h: PLAYER_HEIGHT,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    pub tick: u64,
    // Always sorted by ID, never connection/HashMap iteration order.
    pub players: Vec<Player>,
    pub key_collected: bool,
    pub completed: bool,
    #[serde(default)]
    pub objects: Vec<MechanicState>,
    #[serde(default)]
    pub deaths: u32,
    #[serde(default)]
    pub checkpoint: Option<Rect>,
    #[serde(default)]
    pub level_start_tick: u64,
    /// Tick of the last automatic rescue from an unwinnable split. Clients play
    /// a one-shot "run lost" animation whenever this changes; it is part of the
    /// state hash so two authorities cannot disagree about having rescued.
    #[serde(default)]
    pub rescued_at: Option<u64>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            tick: 0,
            players: Vec::new(),
            key_collected: false,
            completed: false,
            objects: Vec::new(),
            deaths: 0,
            checkpoint: None,
            level_start_tick: 0,
            rescued_at: None,
        }
    }

    pub fn add_player(&mut self, id: PlayerId) -> bool {
        self.add_player_in(&Level::default(), id)
    }

    pub fn add_player_in(&mut self, level: &Level, id: PlayerId) -> bool {
        if !level.valid() {
            return false;
        }
        if self.players.len() >= MAX_PLAYERS || self.players.iter().any(|p| p.id == id) {
            return false;
        }
        self.ensure_objects(level);
        // A checkpoint is a spawn region, not permission to put feet above the
        // world or retain a stale checkpoint from another level.
        self.checkpoint = self.checkpoint.filter(|rect| {
            level
                .mechanics
                .iter()
                .any(|m| m.kind == MechanicKind::Checkpoint && m.rect == *rect)
                && i64::from(rect.y) + i64::from(rect.h) >= i64::from(PLAYER_HEIGHT)
                && i64::from(rect.y) + i64::from(rect.h) <= i64::from(level.height)
        });
        // Find a free spawn column, including after an intermediate player leaves.
        let spawn_y = self
            .checkpoint
            .map_or(level.spawn_y - PLAYER_HEIGHT, |r| r.y + r.h - PLAYER_HEIGHT);
        let spawn_x = self.checkpoint.map_or(0, |r| r.x);
        let Some(x) = (2..=6)
            .map(|column| (spawn_x + column * UNIT).min(level.width - PLAYER_WIDTH))
            // Bounded exhaustive search of the safe starting floor, never an unchecked fallback.
            .chain(spawn_x..=(spawn_x + 8 * UNIT).min(level.width - PLAYER_WIDTH))
            .find(|&x| {
                let rect = Rect {
                    x,
                    y: spawn_y,
                    w: PLAYER_WIDTH,
                    h: PLAYER_HEIGHT,
                };
                rect.x >= 0
                    && rect.y >= 0
                    && rect.x + rect.w <= level.width
                    && rect.y + rect.h <= level.height
                    && !self.players.iter().any(|p| p.rect().overlaps(rect))
                    && !level
                        .solids
                        .iter()
                        .copied()
                        .chain((!self.key_collected).then_some(level.gate))
                        .chain(self.blocking_objects(level))
                        .chain(
                            level
                                .mechanics
                                .iter()
                                .filter(|m| m.kind == MechanicKind::Hazard)
                                .map(|m| m.rect),
                        )
                        .any(|solid| rect.overlaps(solid))
            })
        else {
            return false;
        };
        self.players.push(Player {
            id,
            x,
            y: spawn_y,
            vx: 0,
            vy: 0,
            grounded: true,
            jump_held: false,
        });
        self.players.sort_by_key(|p| p.id);
        true
    }

    pub fn remove_player(&mut self, id: PlayerId) {
        self.players.retain(|p| p.id != id);
    }

    /// One authoritative tick. Missing inputs are neutral. Invalid axes are neutral.
    /// The caller supplies at most one command per player; first occurrence wins.
    pub fn step(&mut self, level: &Level, inputs: &[(PlayerId, Input)]) {
        let commands: Vec<_> = inputs
            .iter()
            .map(|&(id, input)| {
                let held = self
                    .players
                    .iter()
                    .find(|p| p.id == id)
                    .is_some_and(|p| p.jump_held);
                (id, input, input.jump && !held)
            })
            .collect();
        self.step_with_jump_edges(level, &commands, true);
    }

    /// Network entry: jump edges are explicit and never inferred from held state.
    /// Each edge is consumed this tick, including while airborne (no future jump queue).
    /// `ready` prevents disconnected retained participants from causing a new win.
    pub fn step_with_jump_edges(
        &mut self,
        level: &Level,
        inputs: &[(PlayerId, Input, bool)],
        ready: bool,
    ) {
        self.tick = self.tick.saturating_add(1);
        if self.completed {
            return;
        }
        if !level.valid() {
            return;
        }
        self.update_mechanics(level);
        if self.unwinnable_split(level) {
            // The team can no longer reach the key: nobody is left on the pad
            // side to hold a team pad, so its gate can never reopen. Charge the
            // whole team a death and regroup rather than leave the room lost.
            self.rescued_at = Some(self.tick);
            self.respawn_team(level);
            self.update_mechanics(level);
            return;
        }
        let before_tether = (level.tether_length > 0).then(|| self.players.clone());
        // Key collection runs inside the movement loop below, so a rope rollback
        // needs the pre-move value to undo a key gathered on a discarded step.
        let key_before_tether = self.key_collected;
        // Resolve lower players first so a standing player sees its support's new position.
        let mut order: Vec<usize> = (0..self.players.len()).collect();
        order.sort_by_key(|&i| (std::cmp::Reverse(self.players[i].y), self.players[i].id));
        for i in order {
            let id = self.players[i].id;
            // Entering the unlocked exit commits this player to waiting for the
            // team. They no longer body-block the doorway for later arrivals.
            // This is derived from authoritative geometry, not a client claim.
            if self.key_collected && self.players[i].rect().overlaps(level.exit) {
                self.players[i].vx = 0;
                self.players[i].vy = 0;
                self.players[i].jump_held = false;
                continue;
            }
            let (input, jump_edge) = inputs
                .iter()
                .find(|(pid, _, _)| *pid == id)
                .map(|(_, input, edge)| (*input, *edge))
                .filter(|(input, _)| input.valid())
                .unwrap_or_default();
            let obstacles: Vec<Rect> = level
                .solids
                .iter()
                .copied()
                .chain((!self.key_collected).then_some(level.gate))
                .chain(self.blocking_objects(level))
                .chain(
                    self.players
                        .iter()
                        .filter(|p| {
                            p.id != id && !(self.key_collected && p.rect().overlaps(level.exit))
                        })
                        .map(Player::rect),
                )
                .collect();
            let p = &mut self.players[i];
            let on = |r: Rect| {
                p.x < r.x + r.w
                    && p.x + PLAYER_WIDTH > r.x
                    && (p.y + PLAYER_HEIGHT - r.y).abs() <= 32
            };
            let ice = level
                .mechanics
                .iter()
                .any(|m| m.kind == MechanicKind::Ice && (on(m.rect) || p.rect().overlaps(m.rect)));
            let belt: i32 = level
                .mechanics
                .iter()
                .filter(|m| m.kind == MechanicKind::Conveyor && on(m.rect))
                .map(|m| m.travel.clamp(-120, 120))
                .sum();
            let wind: i32 = level
                .mechanics
                .iter()
                .filter(|m| m.kind == MechanicKind::Wind && p.rect().overlaps(m.rect))
                .map(|m| m.travel.clamp(-64, 64))
                .sum();
            p.vx = if ice {
                (p.vx + i32::from(input.axis) * 8).clamp(-144, 144)
            } else {
                i32::from(input.axis) * 96
            };
            p.vx = (p.vx + belt + wind).clamp(-240, 240);
            if jump_edge && p.grounded {
                p.vy = -380;
            }
            // Holding jump climbs a team-tensioned rope; unlike an ordinary
            // jump this is an explicit continuous mechanic, not a repeated edge.
            if input.jump
                && self.objects.iter().any(|o| {
                    o.active
                        && p.rect().overlaps(o.rect)
                        && level
                            .mechanics
                            .iter()
                            .any(|m| m.id == o.id && m.kind == MechanicKind::Rope)
                })
            {
                p.vy = -172;
            }
            p.jump_held = input.jump;
            let low = level
                .mechanics
                .iter()
                .any(|m| m.kind == MechanicKind::LowGravity && p.rect().overlaps(m.rect));
            p.vy = (p.vy + if low { 10 } else { 28 }).min(420);
            let wanted_x = p.x + p.vx;
            let proposed_x = wanted_x.clamp(0, level.width - PLAYER_WIDTH);
            let proposed = Rect {
                x: proposed_x,
                ..p.rect()
            };
            let mut resolved_x = proposed_x;
            for &solid in &obstacles {
                if proposed.overlaps(solid) {
                    if p.vx > 0 {
                        resolved_x = resolved_x.min(solid.x - PLAYER_WIDTH);
                    }
                    if p.vx < 0 {
                        resolved_x = resolved_x.max(solid.x + solid.w);
                    }
                }
            }
            p.x = resolved_x;
            if resolved_x != wanted_x {
                p.vx = 0;
            }
            let proposed_y = p.y + p.vy;
            let proposed = Rect {
                y: proposed_y,
                ..p.rect()
            };
            let mut resolved_y = proposed_y;
            p.grounded = false;
            // Keep the movement direction until every candidate has been considered.
            // Resolving the first hit and zeroing velocity made later surfaces inert.
            let ledges = level
                .mechanics
                .iter()
                .filter(|m| {
                    m.kind == MechanicKind::OneWay && p.vy > 0 && p.y + PLAYER_HEIGHT <= m.rect.y
                })
                .map(|m| m.rect);
            for solid in obstacles.iter().copied().chain(ledges) {
                if proposed.overlaps(solid) {
                    if p.vy > 0 {
                        resolved_y = resolved_y.min(solid.y - PLAYER_HEIGHT);
                        p.grounded = true;
                    } else if p.vy < 0 {
                        resolved_y = resolved_y.max(solid.y + solid.h);
                    }
                }
            }
            p.y = resolved_y;
            if resolved_y != proposed_y {
                p.vy = 0;
            }
            if p.y < 0 {
                p.y = 0;
                p.vy = p.vy.max(0);
            }
            if p.rect().overlaps(level.key) {
                self.key_collected = true;
            }
            if level.mechanics.iter().any(|m| {
                m.kind == MechanicKind::Spring
                    && (p.rect().overlaps(m.rect)
                        || (p.x < m.rect.x + m.rect.w
                            && p.x + PLAYER_WIDTH > m.rect.x
                            && (p.y + PLAYER_HEIGHT - m.rect.y).abs() <= 32))
                    && p.vy >= 0
            }) {
                p.vy = -560;
                p.grounded = false;
            }
        }
        if let Some(previous) = before_tether {
            self.solve_tether(level, previous, key_before_tether);
        }
        if self.players.iter().any(|p| p.rect().overlaps(level.key)) {
            self.key_collected = true;
        }
        if self.players.iter().any(|p| {
            p.y > level.height
                || self.objects.iter().any(|o| {
                    o.active
                        && p.rect().overlaps(o.rect)
                        && level
                            .mechanics
                            .iter()
                            .any(|m| m.id == o.id && m.kind == MechanicKind::FallingRock)
                })
                || level
                    .mechanics
                    .iter()
                    .any(|m| m.kind == MechanicKind::Hazard && touches_spikes(p.rect(), m.rect))
        }) {
            self.respawn_team(level);
            return;
        }
        for m in &level.mechanics {
            if m.kind == MechanicKind::Checkpoint
                && self.players.iter().any(|p| p.rect().overlaps(m.rect))
            {
                self.checkpoint = Some(m.rect);
            }
        }
        self.completed = ready
            && self.key_collected
            && self.players.len() >= usize::from(level.required_players)
            && self.players.iter().all(|p| p.rect().overlaps(level.exit));
    }

    /// Canonical FNV-1a replay checksum, not a cryptographic security primitive.
    pub fn state_hash(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        let mut write = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        };
        write(&self.tick.to_le_bytes());
        // Canonical state version 2: all authoritative mechanic and respawn state.
        write(&2u32.to_le_bytes());
        write(&self.level_start_tick.to_le_bytes());
        write(&self.deaths.to_le_bytes());
        write(&[self.checkpoint.is_some() as u8]);
        if let Some(r) = self.checkpoint {
            for n in [r.x, r.y, r.w, r.h] {
                write(&n.to_le_bytes());
            }
        }
        write(&(self.objects.len() as u32).to_le_bytes());
        for o in &self.objects {
            write(&o.id.to_le_bytes());
            for n in [o.rect.x, o.rect.y, o.rect.w, o.rect.h] {
                write(&n.to_le_bytes());
            }
            write(&[o.active as u8]);
            write(&o.timer.to_le_bytes());
        }
        write(&[self.key_collected as u8, self.completed as u8]);
        write(&self.rescued_at.map_or(u32::MAX, |t| t as u32).to_le_bytes());
        write(&(self.players.len() as u32).to_le_bytes());
        for p in &self.players {
            write(&p.id.to_le_bytes());
            for n in [p.x, p.y, p.vx, p.vy] {
                write(&n.to_le_bytes());
            }
            write(&[p.grounded as u8, p.jump_held as u8]);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_is_identical_every_tick_and_input_order_independent() {
        let level = Level::default();
        let mut a = World::new();
        a.add_player(2);
        a.add_player(1);
        let mut b = a.clone();
        for tick in 0..3600 {
            let inputs = [
                (
                    1,
                    Input {
                        axis: if tick % 120 < 60 { 1 } else { -1 },
                        jump: tick % 45 == 0,
                    },
                ),
                (
                    2,
                    Input {
                        axis: 1,
                        jump: tick % 60 == 0,
                    },
                ),
            ];
            a.step(&level, &inputs);
            b.step(&level, &[inputs[1], inputs[0]]);
            assert_eq!(a.state_hash(), b.state_hash(), "tick {tick}");
        }
    }


    /// Team pad threshold is per-mechanic, not the level's team size, so a
    /// three-player stage can still contain a two-player pad.
    fn team_pad_level(need: i32, required_players: u8) -> Level {
        // Any in-range stage number: `valid()` rejects numbers outside the
        // catalogue, which would stop `step` from initialising objects.
        let mut l = Level {
            number: 28,
            width: 30 * UNIT,
            height: 14 * UNIT,
            spawn_y: 12 * UNIT,
            required_players,
            tether_length: 0,
            title: "team pad test".into(),
            hint: "stand together".into(),
            solids: vec![crate::Rect {
                x: 0,
                y: 12 * UNIT,
                w: 30 * UNIT,
                h: 2 * UNIT,
            }],
            ..Level::default()
        };
        l.mechanics.push(crate::Mechanic {
            id: 1,
            kind: crate::MechanicKind::TeamPad,
            rect: crate::Rect {
                x: 8 * UNIT,
                y: 12 * UNIT - UNIT / 8,
                w: 2 * UNIT,
                h: UNIT / 8,
            },
            travel: need,
            period: 0,
        });
        l.mechanics.push(crate::Mechanic {
            id: 2,
            kind: crate::MechanicKind::PressureGate,
            rect: crate::Rect {
                x: 12 * UNIT,
                y: 8 * UNIT,
                w: UNIT,
                h: 4 * UNIT,
            },
            travel: 0,
            period: 240,
        });
        l
    }

    fn pad_active(world: &World, level: &Level) -> bool {
        world
            .objects
            .iter()
            .zip(&level.mechanics)
            .find(|(_, m)| m.kind == crate::MechanicKind::TeamPad)
            .map(|(o, _)| o.active)
            .expect("team pad object")
    }

    #[test]
    fn team_pad_threshold_survives_a_larger_team() {
        // Three-player stage, two-player pad: two bodies must suffice, which
        // would fail if the threshold were read from required_players.
        let level = team_pad_level(2, 3);
        let mut world = World::for_level(&level);
        for id in 1..=3 {
            world.add_player_in(&level, id);
        }
        // Place two bodies on the pad and one well clear of it, so the test
        // measures the threshold rather than how bodies queue up when walking.
        for (index, player) in world.players.iter_mut().enumerate() {
            player.x = if index < 2 {
                8 * UNIT + index as i32 * PLAYER_WIDTH
            } else {
                2 * UNIT
            };
            player.y = 12 * UNIT - PLAYER_HEIGHT;
            player.vx = 0;
            player.vy = 0;
            player.grounded = true;
        }
        world.step(&level, &[]);
        assert!(
            pad_active(&world, &level),
            "two of three must press a two-player pad"
        );
    }


    #[test]
    fn team_pad_counts_bodies_resting_on_its_edge() {
        // A player standing exactly on the pad's top edge does not overlap the
        // rect, so counting must use the same predicate as the plate check.
        let level = team_pad_level(2, 2);
        let mut world = World::for_level(&level);
        for id in 1..=2 {
            world.add_player_in(&level, id);
        }
        let pad = level
            .mechanics
            .iter()
            .find(|m| m.kind == crate::MechanicKind::TeamPad)
            .expect("team pad")
            .rect;
        for (index, player) in world.players.iter_mut().enumerate() {
            player.x = pad.x + index as i32 * PLAYER_WIDTH;
            player.y = pad.y - PLAYER_HEIGHT;
            player.vx = 0;
            player.vy = 0;
            player.grounded = true;
        }
        world.step(&level, &[]);
        assert!(
            pad_active(&world, &level),
            "two bodies on the pad edge must press it"
        );
    }

    #[test]
    fn a_team_that_all_crossed_is_not_rescued() {
        // If every player is past the gate, they can walk to the key; charging a
        // death and dragging them back would punish a successful crossing.
        let level = crate::campaign::level(29).expect("stage 29");
        let mut world = World::for_level(&level);
        assert!(world.add_player_in(&level, 1));
        assert!(world.add_player_in(&level, 2));
        let gate = level
            .mechanics
            .iter()
            .filter(|m| m.kind == crate::MechanicKind::PressureGate)
            .min_by_key(|m| m.rect.x)
            .expect("gate");
        // Both bodies beyond the door, with the door shut behind them.
        for (index, p) in world.players.iter_mut().enumerate() {
            p.x = gate.rect.x + UNIT + index as i32 * PLAYER_WIDTH;
            p.y = level.spawn_y - PLAYER_HEIGHT;
            p.vx = 0;
            p.vy = 0;
            p.grounded = true;
        }
        for (o, m) in world.objects.iter_mut().zip(&level.mechanics) {
            if m.kind == crate::MechanicKind::PressureGate {
                o.timer = 0;
                o.active = true;
            }
        }
        let deaths = world.deaths;
        world.step(&level, &[]);
        assert!(
            world.rescued_at.is_none(),
            "a team that is entirely past the door has not lost the run"
        );
        assert_eq!(world.deaths, deaths, "no death should be charged");
    }

    #[test]
    fn a_split_team_is_rescued_instead_of_left_unwinnable() {
        // Stage 29: a two-player team pad holds a gate open. Once one player is
        // through and the gate's grace expires, the pad side has one body — one
        // short of the two the pad needs — so the room is lost. The authority
        // must charge a death and regroup rather than let it stay broken.
        let level = crate::campaign::level(29).expect("stage 29");
        let mut world = World::for_level(&level);
        assert!(world.add_player_in(&level, 1));
        assert!(world.add_player_in(&level, 2));
        let pad = level
            .mechanics
            .iter()
            .find(|m| m.kind == crate::MechanicKind::TeamPad)
            .expect("team pad");
        let gate = level
            .mechanics
            .iter()
            .filter(|m| m.kind == crate::MechanicKind::PressureGate)
            .min_by_key(|m| m.rect.x)
            .expect("gate");
        // Straddle the gate: one body on each side, pad unpressed.
        world.players[0].x = pad.rect.x + 2 * UNIT;
        world.players[1].x = gate.rect.x + UNIT;
        for p in &mut world.players {
            p.y = level.spawn_y - PLAYER_HEIGHT;
            p.vx = 0;
            p.vy = 0;
            p.grounded = true;
        }
        let deaths = world.deaths;
        // Drain the gate's release grace so it is genuinely shut.
        for (o, m) in world.objects.iter_mut().zip(&level.mechanics) {
            if m.kind == crate::MechanicKind::PressureGate {
                o.timer = 0;
                o.active = true;
            }
        }
        world.step(&level, &[]);
        assert!(
            world.rescued_at.is_some(),
            "a team that can no longer press the pad must be rescued"
        );
        assert_eq!(world.deaths, deaths + 1, "the rescue charges one death");
        assert!(
            world
                .players
                .iter()
                .all(|p| p.x + PLAYER_WIDTH <= gate.rect.x || world.key_collected),
            "everyone returns to the pad side of the gate"
        );
    }

    #[test]
    fn ground_and_world_bounds_hold() {
        let mut world = World::new();
        world.add_player(1);
        for _ in 0..600 {
            world.step(
                &Level::default(),
                &[(
                    1,
                    Input {
                        axis: -1,
                        jump: false,
                    },
                )],
            );
        }
        assert_eq!(world.players[0].x, 0);
        assert_eq!(world.players[0].y, 12 * UNIT - PLAYER_HEIGHT);
        assert!(world.players[0].grounded);
    }

    #[test]
    fn held_jump_does_not_bunny_hop() {
        let mut world = World::new();
        world.add_player(1);
        for _ in 0..120 {
            world.step(
                &Level::default(),
                &[(
                    1,
                    Input {
                        axis: 0,
                        jump: true,
                    },
                )],
            );
        }
        assert!(world.players[0].grounded);
        world.step(&Level::default(), &[(1, Input::default())]);
        world.step(
            &Level::default(),
            &[(
                1,
                Input {
                    axis: 0,
                    jump: true,
                },
            )],
        );
        assert!(world.players[0].vy < 0);
    }

    #[test]
    fn can_land_on_another_player() {
        let mut world = World::new();
        world.add_player(1);
        world.add_player(2);
        world.players[1].x = world.players[0].x;
        world.players[1].y = world.players[0].y - 2 * PLAYER_HEIGHT;
        for _ in 0..60 {
            world.step(&Level::default(), &[]);
        }
        assert_eq!(world.players[1].y + PLAYER_HEIGHT, world.players[0].y);
        assert!(world.players[1].grounded);
    }

    #[test]
    fn gate_adjacent_landing_chooses_teammate_surface_before_gate() {
        let mut world = World::new();
        world.add_player(1);
        world.add_player(2);
        world.players[0].x = 8544;
        world.players[0].y = 8064;
        world.players[0].vy = 348;
        world.players[0].grounded = false;
        world.players[1].x = 8496;
        world.players[1].y = 9016;
        world.players[1].vy = -44;
        world.players[1].grounded = false;
        assert!(!world.players[0].rect().overlaps(world.players[1].rect()));
        let input = Input {
            axis: 1,
            jump: false,
        };
        world.step(&Level::default(), &[(1, input), (2, input)]);
        assert_eq!(world.players[0].y + PLAYER_HEIGHT, world.players[1].y);
        assert!(!world.players[0].rect().overlaps(world.players[1].rect()));
        assert!(!world.players[0].rect().overlaps(Level::default().gate));
        assert!(world.players[0].grounded);
    }

    #[test]
    fn ordinary_input_stress_keeps_players_out_of_each_other_and_solids() {
        let level = Level::default();
        for seed in 1..=32u64 {
            let mut rng = seed;
            let mut world = World::new();
            for id in 1..=4 {
                world.add_player(id);
            }
            let mut inputs = vec![(0, Input::default()); 4];
            for tick in 0..2000 {
                if tick % 15 == 0 {
                    for (i, command) in inputs.iter_mut().enumerate() {
                        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                        *command = (
                            i as u32 + 1,
                            Input {
                                axis: ((rng >> 32) % 3) as i8 - 1,
                                jump: (rng >> 40) & 1 == 1,
                            },
                        );
                    }
                }
                world.step(&level, &inputs);
                for (i, player) in world.players.iter().enumerate() {
                    for other in &world.players[i + 1..] {
                        assert!(
                            !player.rect().overlaps(other.rect()),
                            "seed {seed} tick {tick}"
                        );
                    }
                    for solid in level
                        .solids
                        .iter()
                        .copied()
                        .chain((!world.key_collected).then_some(level.gate))
                    {
                        assert!(!player.rect().overlaps(solid), "seed {seed} tick {tick}");
                    }
                }
            }
        }
    }

    #[test]
    fn wall_prevents_walking_through() {
        let mut world = World::new();
        world.add_player(1);
        for _ in 0..200 {
            world.step(
                &Level::default(),
                &[(
                    1,
                    Input {
                        axis: 1,
                        jump: false,
                    },
                )],
            );
        }
        assert_eq!(world.players[0].x + PLAYER_WIDTH, 9 * UNIT);
    }

    #[test]
    fn spike_collision_matches_teeth_not_empty_upper_corners() {
        let hazard = Rect {
            x: 1000,
            y: 1000,
            w: 300,
            h: 300,
        };
        assert!(!touches_spikes(
            Rect {
                x: 1000,
                y: 1000,
                w: 10,
                h: 10
            },
            hazard
        ));
        assert!(touches_spikes(
            Rect {
                x: 1145,
                y: 1000,
                w: 10,
                h: 10
            },
            hazard
        ));
        assert!(touches_spikes(
            Rect {
                x: 1000,
                y: 1290,
                w: 20,
                h: 10
            },
            hazard
        ));
        assert!(!touches_spikes(
            Rect {
                x: 900,
                y: 1000,
                w: 100,
                h: 300
            },
            hazard
        ));
        let wide = Rect { w: 1050, ..hazard }; // Three teeth, including valleys.
        assert!(!touches_spikes(
            Rect {
                x: 1345,
                y: 1000,
                w: 10,
                h: 10
            },
            wide
        ));
        assert!(touches_spikes(
            Rect {
                x: 1345,
                y: 1290,
                w: 10,
                h: 10
            },
            wide
        ));
    }

    #[test]
    fn first_arrival_cannot_block_any_teammate_at_exit() {
        let level = Level::default();
        for first in [1, 2, 3, 4] {
            let mut world = World::for_level(&level);
            for id in 1..=4 {
                assert!(world.add_player_in(&level, id));
            }
            world.key_collected = true;
            for (index, p) in world.players.iter_mut().enumerate() {
                p.x = level.exit.x - PLAYER_WIDTH - (index as i32 + 1) * UNIT;
                p.y = level.exit.y;
            }
            let leader = world.players.iter_mut().find(|p| p.id == first).unwrap();
            leader.x = level.exit.x - PLAYER_WIDTH + 1; // Stop at the very lip.
            leader.y = level.exit.y;
            for _ in 0..240 {
                world.step(
                    &level,
                    &(1..=4)
                        .map(|id| {
                            (
                                id,
                                Input {
                                    axis: 1,
                                    jump: false,
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                );
                if world.completed {
                    break;
                }
            }
            assert!(world.completed, "first P{first}: {:?}", world.players);
        }
    }

    #[test]
    fn completion_requires_key_and_everyone_at_exit() {
        let mut world = World::new();
        world.add_player(1);
        world.add_player(2);
        world.players[0].x = 22 * UNIT;
        world.step(&Level::default(), &[]);
        assert!(!world.completed);
        world.key_collected = true;
        world.step(&Level::default(), &[]);
        assert!(!world.completed);
        world.players[1].x = 23 * UNIT - PLAYER_WIDTH;
        world.step(&Level::default(), &[]);
        assert!(world.completed);
    }

    #[test]
    fn duplicate_capacity_and_disconnect_are_safe() {
        let mut world = World::new();
        for id in 1..=4 {
            assert!(world.add_player(id));
        }
        assert!(!world.add_player(5));
        assert!(!world.add_player(1));
        world.remove_player(2);
        assert!(world.add_player(5));
        assert!(!world.players[1].rect().overlaps(world.players[2].rect()));
    }

    #[test]
    fn obstructed_spawn_columns_use_checked_search() {
        let mut world = World::new();
        for id in 1..=3 {
            assert!(world.add_player(id));
        }
        for (p, x) in world.players.iter_mut().zip([1568, 3552, 5632]) {
            p.x = x;
        }
        assert!(world.add_player(4));
        for (i, p) in world.players.iter().enumerate() {
            assert!(p.x >= 0 && p.x + PLAYER_WIDTH <= Level::default().width);
            for q in &world.players[i + 1..] {
                assert!(!p.rect().overlaps(q.rect()));
            }
        }
    }

    #[test]
    fn two_players_can_complete_the_level_without_teleporting() {
        let mut world = World::new();
        let level = Level::default();
        world.add_player(1);
        world.add_player(2);
        let right = Input {
            axis: 1,
            jump: false,
        };
        let jump_right = Input {
            axis: 1,
            jump: true,
        };
        for _ in 0..200 {
            world.step(&level, &[(2, right)]);
        }
        for _ in 0..52 {
            world.step(&level, &[(1, right)]);
        }
        for _ in 0..50 {
            world.step(&level, &[(1, jump_right)]);
        }
        assert_eq!(world.players[0].y + PLAYER_HEIGHT, world.players[1].y);
        world.step(&level, &[]);
        for _ in 0..60 {
            world.step(&level, &[(1, jump_right)]);
        }
        assert!(world.key_collected, "teammate-assisted jump must reach key");
        for _ in 0..200 {
            let inputs: Vec<_> = world
                .players
                .iter()
                .map(|p| {
                    (
                        p.id,
                        Input {
                            axis: if p.x < 22 * UNIT + UNIT / 2 { 1 } else { 0 },
                            jump: false,
                        },
                    )
                })
                .collect();
            world.step(&level, &inputs);
        }
        assert!(world.completed);
        let expected: u64 = include_str!("../../../tests/golden/coop-v1.hash")
            .trim()
            .parse()
            .unwrap();
        assert_eq!(world.state_hash(), expected, "golden replay changed");
    }
}
