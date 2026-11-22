use super::*;

fn team(l: &Level) -> World {
    let mut w = World::for_level(l);
    assert!(w.add_player_in(l, 1));
    assert!(w.add_player_in(l, 2));
    w
}

fn only(kind: MechanicKind, rect: Rect, travel: i32, period: u32) -> Level {
    let mut l = level(4).unwrap();
    l.mechanics.clear();
    add(&mut l, kind, rect, travel, period);
    l
}

#[test]
fn catalog_is_bounded_distinct_and_curated() {
    assert!(level(0).is_none());
    assert!(level(LEVEL_COUNT + 1).is_none());
    let levels: Vec<_> = (1..=LEVEL_COUNT).map(|n| level(n).unwrap()).collect();
    for (i, l) in levels.iter().enumerate() {
        assert!(l.valid(), "level {}", l.number);
        assert_eq!(l.number as usize, i + 1);
        assert!(!l.title.is_empty() && !l.hint.is_empty());
        assert_eq!(team(l).players.len(), 2);
        for other in &levels[..i] {
            assert!(
                l.solids != other.solids
                    || l.gate != other.gate
                    || l.key != other.key
                    || l.mechanics != other.mechanics
            );
        }
    }
    // Each lesson has a different primary feature. Hazards may appear earlier
    // underneath a ferry before their dedicated precision-jump lesson.
    let primary = [
        MechanicKind::Plate,
        MechanicKind::TimedGate,
        MechanicKind::OneWay,
        MechanicKind::MovingPlatform,
        MechanicKind::Conveyor,
        MechanicKind::Ice,
        MechanicKind::Spring,
        MechanicKind::Hazard,
        MechanicKind::Checkpoint,
        MechanicKind::Crumble,
        MechanicKind::LowGravity,
        MechanicKind::Wind,
    ];
    let mut seen = Vec::new();
    for (l, kind) in levels[3..15].iter().zip(primary) {
        assert!(l.mechanics.iter().any(|m| m.kind == kind));
        for m in &l.mechanics {
            if !seen.contains(&m.kind) {
                seen.push(m.kind);
            }
        }
    }
    assert_eq!(seen.len(), 13); // plate + gate form one of twelve new features.
    for l in &levels[15..] {
        assert!(l.mechanics.len() >= 4 || l.solids.len() >= 15);
    }
}

#[test]
fn plate_opens_gate_with_release_grace() {
    let l = level(4).unwrap();
    let mut w = team(&l);
    assert!(w.objects[1].active);
    w.players[0].x = 8 * UNIT;
    w.step(&l, &[]);
    assert!(w.objects[0].active);
    assert!(!w.objects[1].active);
    w.players[0].x = 2 * UNIT;
    for _ in 0..239 {
        w.step(&l, &[]);
    }
    assert!(!w.objects[1].active);
    w.step(&l, &[]);
    assert!(w.objects[1].active);
}

#[test]
fn timed_gate_waits_to_close_until_clear() {
    let l = only(MechanicKind::TimedGate, r(12, 8, 1, 4), 0, 120);
    let mut w = team(&l);
    for _ in 0..60 {
        w.step(&l, &[]);
    }
    assert!(!w.objects[0].active);
    w.players[0].x = 12 * UNIT;
    for _ in 0..60 {
        w.step(&l, &[]);
    }
    assert!(!w.objects[0].active);
    w.players[0].x = 8 * UNIT;
    w.step(&l, &[]);
    assert!(w.objects[0].active);
}

#[test]
fn one_way_allows_ascent_and_catches_descent() {
    let l = only(
        MechanicKind::OneWay,
        Rect {
            h: UNIT / 4,
            ..r(8, 10, 4, 1)
        },
        0,
        0,
    );
    let mut w = team(&l);
    w.players[0].x = 9 * UNIT;
    w.step(
        &l,
        &[(
            1,
            Input {
                axis: 0,
                jump: true,
            },
        )],
    );
    for _ in 0..60 {
        w.step(&l, &[]);
    }
    assert_eq!(w.players[0].y + PLAYER_HEIGHT, 10 * UNIT);
    assert!(w.players[0].grounded);
}

#[test]
fn horizontal_shuttle_carries_rider_and_stops_at_blocker() {
    let mut l = only(
        MechanicKind::MovingPlatform,
        Rect {
            h: UNIT / 2,
            ..r(8, 10, 3, 1)
        },
        2 * UNIT,
        240,
    );
    let mut w = team(&l);
    w.players[0].x = 9 * UNIT;
    w.players[0].y = 10 * UNIT - PLAYER_HEIGHT;
    let x = w.players[0].x;
    for _ in 0..80 {
        w.step(&l, &[]);
    }
    assert!(w.players[0].x > x + UNIT);
    assert_eq!(w.players[0].y + PLAYER_HEIGHT, w.objects[0].rect.y);
    l.solids.push(r(12, 7, 1, 5));
    for _ in 0..500 {
        w.step(&l, &[]);
        for p in &w.players {
            assert!(!p.rect().overlaps(w.objects[0].rect));
            assert!(!p.rect().overlaps(*l.solids.last().unwrap()));
        }
    }
}

#[test]
fn conveyor_ice_wind_and_low_gravity_change_motion() {
    let mut l = only(MechanicKind::Conveyor, r(1, 12, 8, 1), 48, 0);
    let mut w = team(&l);
    let x = w.players[0].x;
    w.step(&l, &[]);
    assert_eq!(w.players[0].x, x + 48);
    l.mechanics[0].kind = MechanicKind::Ice;
    w.players[0].vx = 100;
    w.step(&l, &[]);
    assert_eq!(w.players[0].vx, 100);
    l.mechanics[0].kind = MechanicKind::Wind;
    l.mechanics[0].rect = r(0, 0, 10, 12);
    w.step(&l, &[]);
    assert_eq!(w.players[0].vx, 48);
    l.mechanics[0].kind = MechanicKind::LowGravity;
    w.players[0].y = 8 * UNIT;
    w.players[0].vy = 0;
    w.players[0].grounded = false;
    w.step(&l, &[]);
    assert_eq!(w.players[0].vy, 10);
}

#[test]
fn spring_launches_without_a_jump_command() {
    let l = level(10).unwrap();
    let mut w = team(&l);
    w.players[0].x = 11 * UNIT;
    w.step(&l, &[]);
    assert_eq!(w.players[0].vy, -560);
    for _ in 0..30 {
        w.step(&l, &[]);
    }
    assert!(w.key_collected);
}

#[test]
fn crumble_breaks_and_rebuilds_without_embedding() {
    let l = only(
        MechanicKind::Crumble,
        Rect {
            h: UNIT / 2,
            ..r(8, 10, 4, 1)
        },
        0,
        20,
    );
    let mut w = team(&l);
    w.players[0].x = 9 * UNIT;
    w.players[0].y = 10 * UNIT - PLAYER_HEIGHT;
    for _ in 0..21 {
        w.step(&l, &[]);
    }
    assert!(!w.objects[0].active);
    for _ in 0..220 {
        w.step(&l, &[]);
    }
    assert!(w.objects[0].active);
    assert!(!w.players[0].rect().overlaps(w.objects[0].rect));
}

#[test]
fn hazard_respawns_team_atomically_at_checkpoint_and_keeps_key() {
    let l = level(12).unwrap();
    let mut w = team(&l);
    w.players[0].x = 7 * UNIT;
    w.step(&l, &[]);
    assert!(w.checkpoint.is_some());
    w.key_collected = true;
    w.players[1].x = 13 * UNIT;
    w.step(&l, &[]);
    assert_eq!(w.deaths, 1);
    assert_eq!(w.players.len(), 2);
    assert!(w.key_collected);
    assert!(w.players.iter().all(|p| p.x >= 7 * UNIT && p.x < 13 * UNIT));
    assert!(!w.players[0].rect().overlaps(w.players[1].rect()));
}

#[test]
fn bad_level_parameters_fail_closed_without_panics() {
    let mut l = level(7).unwrap();
    l.width = i32::MIN;
    let mut w = World::for_level(&l);
    assert!(!w.add_player_in(&l, 1));
    w.step(&l, &[]);
    assert_eq!(w.tick, 1);
    l = level(7).unwrap();
    l.mechanics[1].travel = i32::MIN;
    assert!(!l.valid());
    l.mechanics[1].travel = UNIT;
    l.mechanics[1].period = u32::MAX;
    assert!(!l.valid());
    l.mechanics[1].period = 120;
    l.solids[0].w = i32::MAX;
    assert!(!l.valid());
    assert!(!Rect {
        x: i32::MAX,
        y: 0,
        w: i32::MAX,
        h: 1
    }
    .overlaps(r(0, 0, 1, 1)));
}

#[test]
fn all_authoritative_fields_affect_hash() {
    let w = team(&level(7).unwrap());
    let original = w.state_hash();
    let mut variants = Vec::new();
    let mut a = w.clone();
    a.objects[0].active = !a.objects[0].active;
    variants.push(a);
    let mut a = w.clone();
    a.objects[0].timer += 1;
    variants.push(a);
    let mut a = w.clone();
    a.objects[0].rect.x += 1;
    variants.push(a);
    let mut a = w.clone();
    a.objects[0].id += 1;
    variants.push(a);
    let mut a = w.clone();
    a.checkpoint = Some(r(1, 10, 4, 2));
    variants.push(a);
    let mut a = w.clone();
    a.deaths += 1;
    variants.push(a);
    let mut a = w.clone();
    a.level_start_tick += 1;
    variants.push(a);
    for a in variants {
        assert_ne!(a.state_hash(), original);
    }
}

#[test]
fn campaign_replays_are_deterministic_and_collision_safe() {
    for n in 1..=LEVEL_COUNT {
        let l = level(n).unwrap();
        let mut a = team(&l);
        assert!(a.add_player_in(&l, 3));
        assert!(a.add_player_in(&l, 4));
        let mut b = a.clone();
        let mut rng = u64::from(n);
        let mut inputs = [
            (1, Input::default()),
            (2, Input::default()),
            (3, Input::default()),
            (4, Input::default()),
        ];
        for tick in 0..2400 {
            if tick % 17 == 0 {
                for (_, input) in &mut inputs {
                    rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                    *input = Input {
                        axis: (((rng >> 32) % 4) as i8 - 1).min(1),
                        jump: rng >> 63 != 0,
                    };
                }
            }
            a.step(&l, &inputs);
            b.step(&l, &[inputs[3], inputs[2], inputs[1], inputs[0]]);
            assert_eq!(a.state_hash(), b.state_hash(), "level {n} tick {tick}");
            for (i, p) in a.players.iter().enumerate() {
                assert!(p.x >= 0 && p.x + PLAYER_WIDTH <= l.width);
                for q in &a.players[i + 1..] {
                    assert!(
                        !p.rect().overlaps(q.rect())
                            || (a.key_collected
                                && (p.rect().overlaps(l.exit) || q.rect().overlaps(l.exit))),
                        "players level {n} tick {tick}"
                    );
                }
                for solid in l
                    .solids
                    .iter()
                    .copied()
                    .chain((!a.key_collected).then_some(l.gate))
                    .chain(a.blocking_objects(&l))
                {
                    assert!(!p.rect().overlaps(solid), "solid level {n} tick {tick}");
                }
            }
        }
    }
}

#[test]
fn input_only_completion_of_advanced_relay() {
    let l = level(16).unwrap();
    let mut w = ReviewWorld::new(&l);
    for _ in 0..1400 {
        let commands: Vec<_> = w
            .players
            .iter()
            .map(|p| {
                let near_hazard = p.x + PLAYER_WIDTH >= 25 * UNIT - UNIT / 4 && p.x < 26 * UNIT;
                (
                    p.id,
                    Input {
                        axis: if p.x < l.exit.x + UNIT { 1 } else { 0 },
                        jump: near_hazard && p.grounded && !p.jump_held,
                    },
                )
            })
            .collect();
        w.step(&l, &commands);
        if w.completed {
            break;
        }
    }
    assert!(w.completed, "players {:?}, deaths {}", w.players, w.deaths);
    assert_eq!(w.deaths, 0);
}

#[test]
fn input_only_completion_of_bounce_low_gravity_remix() {
    let l = level(18).unwrap();
    let mut w = ReviewWorld::new(&l);
    for _ in 0..2200 {
        let commands: Vec<_> = w
            .players
            .iter()
            .map(|p| {
                let target = if w.key_collected {
                    l.exit.x + UNIT
                } else {
                    l.key.x
                };
                let near_hazard =
                    p.x + PLAYER_WIDTH >= 23 * UNIT - UNIT / 4 && p.x < 24 * UNIT + UNIT / 4;
                (
                    p.id,
                    Input {
                        axis: if p.x < target - 48 {
                            1
                        } else if p.x > target + 48 {
                            -1
                        } else {
                            0
                        },
                        jump: w.key_collected && near_hazard && p.grounded && !p.jump_held,
                    },
                )
            })
            .collect();
        w.step(&l, &commands);
        if w.completed {
            break;
        }
    }
    assert!(
        w.completed,
        "key {}, players {:?}, deaths {}",
        w.key_collected, w.players, w.deaths
    );
    assert_eq!(w.deaths, 0);
}

#[test]
fn input_only_completion_of_floor_and_high_key_lessons() {
    for n in [4, 5, 8, 9, 10, 11, 12, 14, 15] {
        let l = level(n).unwrap();
        let mut w = ReviewWorld::new(&l);
        for _ in 0..2200 {
            let commands: Vec<_> = w
                .players
                .iter()
                .map(|p| {
                    let high_key = matches!(n, 10 | 14) && !w.key_collected;
                    let target = if high_key { l.key.x } else { l.exit.x + UNIT };
                    let hazard_jump = l.mechanics.iter().any(|m| {
                        m.kind == MechanicKind::Hazard
                            && p.x + PLAYER_WIDTH >= m.rect.x - UNIT / 4
                            && p.x < m.rect.x + m.rect.w
                    });
                    let light_jump = n == 14 && !w.key_collected && p.x >= 8 * UNIT;
                    let step_jump = l.solids.iter().any(|r| {
                        r.y < 12 * UNIT
                            && p.y + PLAYER_HEIGHT > r.y
                            && p.x + PLAYER_WIDTH >= r.x - UNIT / 4
                            && p.x < r.x + r.w
                    });
                    (
                        p.id,
                        Input {
                            axis: if p.x < target - 48 {
                                1
                            } else if p.x > target + 48 {
                                -1
                            } else {
                                0
                            },
                            jump: (hazard_jump || light_jump || step_jump)
                                && p.grounded
                                && !p.jump_held,
                        },
                    )
                })
                .collect();
            w.step(&l, &commands);
            if w.completed {
                break;
            }
        }
        assert!(
            w.completed,
            "level {n}, key {}, players {:?}, deaths {}",
            w.key_collected, w.players, w.deaths
        );
        assert_eq!(w.deaths, 0, "level {n}");
    }
}

#[test]
fn input_only_completion_of_three_cooperative_foundations() {
    for n in 1..=3 {
        let l = level(n).unwrap();
        let mut w = ReviewWorld::new(&l);
        let right = Input {
            axis: 1,
            jump: false,
        };
        let jump_right = Input {
            axis: 1,
            jump: true,
        };
        let step_jump = |p: &Player| {
            p.grounded
                && !p.jump_held
                && l.solids.iter().any(|r| {
                    r.y < 12 * UNIT
                        && p.y + PLAYER_HEIGHT > r.y
                        && p.x + PLAYER_WIDTH >= r.x - UNIT / 4
                        && p.x < r.x + r.w
                })
        };
        for _ in 0..200 {
            w.step(
                &l,
                &[(
                    2,
                    Input {
                        jump: step_jump(&w.players[1]),
                        ..right
                    },
                )],
            );
        }
        for _ in 0..160 {
            let p = &w.players[0];
            w.step(
                &l,
                &[(
                    1,
                    Input {
                        axis: i8::from(p.x < l.gate.x - 17 * UNIT / 8),
                        jump: step_jump(p),
                    },
                )],
            );
        }
        for _ in 0..50 {
            w.step(&l, &[(1, jump_right)]);
        }
        w.step(&l, &[]);
        for _ in 0..60 {
            w.step(&l, &[(1, jump_right)]);
        }
        assert!(w.key_collected, "foundation {n}");
        for _ in 0..300 {
            let commands: Vec<_> = w
                .players
                .iter()
                .map(|p| {
                    (
                        p.id,
                        Input {
                            axis: i8::from(p.x < l.exit.x + UNIT / 2),
                            jump: step_jump(p),
                        },
                    )
                })
                .collect();
            w.step(&l, &commands);
        }
        assert!(w.completed, "foundation {n}, {:?}", w.players);
        assert_eq!(w.deaths, 0);
    }
}

#[test]
fn restart_can_preserve_network_tick_without_inheriting_gate_phase() {
    let l = level(5).unwrap();
    let mut w = World::for_level(&l);
    w.tick = 10_123;
    w.level_start_tick = w.tick;
    assert!(w.add_player_in(&l, 7));
    for _ in 0..119 {
        w.step(&l, &[]);
    }
    assert!(w.objects[0].active);
    w.step(&l, &[]);
    assert!(!w.objects[0].active);
    assert_eq!(w.tick, 10_243);
}

// Completion witnesses below only inspect snapshots and submit ordinary Input.
// No player coordinates, world flags, objects, or catalog data are mutated.
struct ReviewWorld {
    world: World,
    replay: World,
    level: Level,
    frames: Vec<World>,
}

impl std::ops::Deref for ReviewWorld {
    type Target = World;
    fn deref(&self) -> &World {
        &self.world
    }
}
impl ReviewWorld {
    fn new(level: &Level) -> Self {
        let mut world = team(level);
        for id in 3..=u32::from(level.required_players) {
            assert!(world.add_player_in(level, id));
        }
        Self {
            replay: world.clone(),
            frames: vec![world.clone()],
            world,
            level: level.clone(),
        }
    }
    fn step(&mut self, level: &Level, inputs: &[(PlayerId, Input)]) {
        if self.world.completed {
            return;
        }
        self.world.step(level, inputs);
        self.replay.step(level, inputs);
        assert_eq!(self.world.state_hash(), self.replay.state_hash());
        if self
            .world
            .tick
            .is_multiple_of(if level.number >= 25 { 6 } else { 3 })
            || self.world.completed
        {
            self.frames.push(self.world.clone());
        }
    }
}

#[test]
fn expeditions_require_real_rope_partners_and_declared_team_sizes() {
    for number in [23, 24] {
        let l = level(number).unwrap();
        assert!(l.width >= 80 * UNIT && l.height >= 40 * UNIT);
        let mut w = team(&l);
        for (index, p) in w.players.iter_mut().enumerate() {
            p.x = (9 + 2 * index as i32) * UNIT;
        }
        for _ in 0..60 {
            w.step(
                &l,
                &[
                    (
                        1,
                        Input {
                            axis: 0,
                            jump: true,
                        },
                    ),
                    (
                        2,
                        Input {
                            axis: 0,
                            jump: true,
                        },
                    ),
                ],
            );
        }
        assert!(!w.objects[0].active);
        assert!(!w.key_collected);
        w.key_collected = true;
        for p in &mut w.players {
            p.x = l.exit.x;
            p.y = l.exit.y;
        }
        w.step(&l, &[]);
        assert!(!w.completed);
    }
}

#[test]
fn input_only_completion_of_long_platform_courses() {
    for number in 25..=LEVEL_COUNT {
        let l = level(number).unwrap();
        let mut w = ReviewWorld::new(&l);
        let count = u32::from(l.required_players);
        let route = l
            .solids
            .iter()
            .copied()
            .chain(
                l.mechanics
                    .iter()
                    .filter(|m| m.kind == MechanicKind::OneWay)
                    .map(|m| m.rect),
            )
            .collect::<Vec<_>>();
        for index in 1..route.len() {
            let source = route[index - 1];
            let target = route[index];
            let direction = (target.x - source.x).signum() as i8;
            let mut landed = std::collections::BTreeSet::new();
            for _ in 0..600 {
                if w.completed {
                    break;
                }
                for p in &w.players {
                    if p.grounded
                        && p.y + PLAYER_HEIGHT == target.y
                        && p.x + PLAYER_WIDTH > target.x
                        && p.x < target.x + target.w
                    {
                        landed.insert(p.id);
                    }
                }
                if landed.len() == count as usize
                    && w.players
                        .iter()
                        .all(|p| p.grounded && p.y + PLAYER_HEIGHT == target.y)
                {
                    break;
                }
                let commands = w
                    .players
                    .iter()
                    .map(|p| {
                        let goal = target.x
                            + (if direction > 0 {
                                p.id
                            } else {
                                5 - count + p.id
                            }) as i32
                                * UNIT;
                        let jump = !landed.contains(&p.id)
                            && p.grounded
                            && !p.jump_held
                            && p.y + PLAYER_HEIGHT == source.y
                            && (if direction > 0 {
                                p.x + PLAYER_WIDTH >= source.x + source.w - 48
                            } else {
                                p.x <= source.x + 48
                            });
                        let axis = if landed.contains(&p.id) {
                            if (p.x - goal).abs() < 96 {
                                0
                            } else {
                                (goal - p.x).signum() as i8
                            }
                        } else {
                            direction
                        };
                        (p.id, Input { axis, jump })
                    })
                    .collect::<Vec<_>>();
                w.step(&l, &commands);
            }
            assert_eq!(
                w.deaths, 0,
                "course {number}, ledge {index}: {:?}",
                w.players
            );
            assert!(
                w.completed
                    || w.players
                        .iter()
                        .all(|p| p.grounded && p.y + PLAYER_HEIGHT == target.y),
                "course {number}, ledge {index}: {:?}",
                w.players
            );
            if w.completed {
                break;
            }
        }
        for _ in 0..300 {
            w.step(
                &l,
                &(1..=count)
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
            if w.completed {
                break;
            }
        }
        assert!(w.completed, "finish course {number}: {:?}", w.players);
        assert_eq!(w.deaths, 0);
    }
}

#[test]
fn input_only_completion_of_large_team_expeditions() {
    for number in 21..=24 {
        let l = level(number).unwrap();
        let mut w = ReviewWorld::new(&l);
        let count = u32::from(l.required_players);
        for id in (1..=count).rev() {
            walk_to(&l, &mut w, id, (9 + 2 * (id as i32 - 1)) * UNIT);
        }
        hold(&l, &mut w, 3);
        assert!(
            !w.objects
                .iter()
                .find(|o| l
                    .mechanics
                    .iter()
                    .any(|m| m.id == o.id && m.kind == MechanicKind::PressureGate))
                .unwrap()
                .active
        );
        for _ in 0..1000 {
            w.step(
                &l,
                &(1..=count)
                    .map(|id| {
                        (
                            id,
                            Input {
                                axis: 0,
                                jump: true,
                            },
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            if w.key_collected {
                break;
            }
        }
        assert!(w.key_collected, "rope key {number}: {:?}", w.players);
        for rock in l
            .mechanics
            .iter()
            .filter(|m| m.kind == MechanicKind::FallingRock)
        {
            let lead = rock.rect.x - 2 * UNIT;
            for _ in 0..1400 {
                let commands = w
                    .players
                    .iter()
                    .map(|p| {
                        (
                            p.id,
                            steering(&l, p, lead - 2 * (count - p.id) as i32 * UNIT),
                        )
                    })
                    .collect::<Vec<_>>();
                let reached = w.players.iter().all(|p| {
                    (p.x - (lead - 2 * (count - p.id) as i32 * UNIT)).abs() <= 96 && p.grounded
                });
                if reached {
                    break;
                }
                w.step(&l, &commands);
            }
            for _ in 0..250 {
                if w.objects.iter().find(|o| o.id == rock.id).unwrap().timer < 5 {
                    break;
                }
                w.step(&l, &[]);
            }
            // Cross as a group during the warning, well before the rock reaches
            // floor height. Ordinary input only, no timer/state manipulation.
            for _ in 0..110 {
                w.step(
                    &l,
                    &(1..=count)
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
            }
            assert_eq!(w.deaths, 0, "quarry {number}: {:?}", w.players);
        }
        for _ in 0..1600 {
            w.step(
                &l,
                &(1..=count)
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
            if w.completed {
                break;
            }
        }
        assert!(w.completed, "expedition {number}: {:?}", w.players);
        assert_eq!(w.deaths, 0);
    }
}
impl Drop for ReviewWorld {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        assert!(self.world.completed && self.world.deaths == 0);
        let data = serde_json::json!({
            "format": 1, "tick_hz": TICK_HZ, "frame_stride": if self.level.number >= 25 { 6 } else { 3 },
                "provenance": "death-free input-only deterministic simulation witness; not a proven optimal route",
                "level": self.level, "frames": self.frames,
                "final_hash": format!("{:016x}", self.world.state_hash()),
        });
        if let Ok(directory) = std::env::var("COOP_EXPORT_SOLUTIONS") {
            let directory = std::path::Path::new(&directory);
            std::fs::create_dir_all(directory).unwrap();
            std::fs::write(
                directory.join(format!("{}.json", self.level.number)),
                serde_json::to_vec(&data).unwrap(),
            )
            .unwrap();
        } else {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../web/solutions/{}.json", self.level.number));
            let checked_in: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            // Compare without dumping entire recordings on failure.
            assert!(
                data == checked_in,
                "solution {} is stale: regenerate recordings",
                self.level.number
            );
        }
    }
}

fn steering(l: &Level, p: &Player, target: i32) -> Input {
    let on_ice = l.mechanics.iter().any(|m| {
        m.kind == MechanicKind::Ice
            && p.y + PLAYER_HEIGHT == m.rect.y
            && p.x < m.rect.x + m.rect.w
            && p.x + PLAYER_WIDTH > m.rect.x
    });
    let delta = target - p.x;
    Input {
        // Brake ahead of the mark on ice; do not pretend neutral input stops it.
        axis: if on_ice {
            (delta - p.vx * 9).signum() as i8
        } else if delta.abs() <= 48 {
            0
        } else {
            delta.signum() as i8
        },
        jump: false,
    }
}

fn walk_to(l: &Level, w: &mut ReviewWorld, id: PlayerId, target: i32) {
    for _ in 0..700 {
        if w.completed {
            return;
        }
        let p = w.players.iter().find(|p| p.id == id).unwrap();
        if w.key_collected && p.rect().overlaps(l.exit) {
            return;
        }
        let delta = target - p.x;
        if delta.abs() <= 96 && p.vx.abs() <= 96 {
            return;
        }
        let input = steering(l, p, target);
        w.step(l, &[(id, input)]);
    }
    panic!(
        "level {}, walking {id} to {target}: {:?}",
        l.number, w.players
    );
}

fn hold(l: &Level, w: &mut ReviewWorld, ticks: u32) {
    for _ in 0..ticks {
        w.step(l, &[]);
    }
}

fn run(l: &Level, w: &mut ReviewWorld, id: PlayerId, ticks: u32, jump: bool) {
    for _ in 0..ticks {
        w.step(l, &[(id, Input { axis: 1, jump })]);
    }
}

#[test]
fn input_only_completion_of_one_way_staircase() {
    let l = level(6).unwrap();
    let mut w = ReviewWorld::new(&l);
    walk_to(&l, &mut w, 2, 9 * UNIT);
    w.step(
        &l,
        &[(
            2,
            Input {
                axis: 0,
                jump: true,
            },
        )],
    );
    hold(&l, &mut w, 40);
    assert_eq!(w.players[1].y + PLAYER_HEIGHT, 10 * UNIT);
    walk_to(&l, &mut w, 2, 10 * UNIT + UNIT / 4);
    run(&l, &mut w, 2, 30, true);
    hold(&l, &mut w, 30);
    assert!(w.key_collected, "{:?}", w.players);
    assert_eq!(w.players[1].y + PLAYER_HEIGHT, 8 * UNIT);
    walk_to(&l, &mut w, 2, l.exit.x + UNIT);
    walk_to(&l, &mut w, 1, l.exit.x);
    hold(&l, &mut w, 30);
    assert!(w.completed);
    assert_eq!(w.deaths, 0);
}

#[test]
fn input_only_completion_of_crumbling_bridges() {
    for n in [13, 19] {
        let l = level(n).unwrap();
        let mut w = ReviewWorld::new(&l);
        let bridge = l
            .mechanics
            .iter()
            .find(|m| m.kind == MechanicKind::Crumble)
            .unwrap();
        for id in [2, 1] {
            walk_to(&l, &mut w, id, bridge.rect.x - UNIT);
            run(&l, &mut w, id, 100, true);
            assert_eq!(w.deaths, 0, "bridge {n}: {:?}", w.players);
            let p = w.players.iter().find(|p| p.id == id).unwrap();
            let danger = l
                .mechanics
                .iter()
                .find(|m| m.kind == MechanicKind::Hazard)
                .unwrap()
                .rect;
            assert!(p.x >= danger.x + danger.w, "bridge {n}: {p:?}");
            if id == 2 {
                hold(&l, &mut w, 300);
            }
        }
        // Walking naturally activates the pressure pad and waits at the timed
        // door in the remix; no direct manipulation of gate state is needed.
        for _ in 0..800 {
            let commands: Vec<_> = w
                .players
                .iter()
                .map(|p| {
                    (
                        p.id,
                        Input {
                            axis: i8::from(p.x < l.exit.x + UNIT),
                            jump: false,
                        },
                    )
                })
                .collect();
            w.step(&l, &commands);
            if w.completed {
                break;
            }
        }
        assert!(w.completed, "bridge {n}: {:?}", w.players);
        assert_eq!(w.deaths, 0);
    }
}

fn cross_ferry(l: &Level, w: &mut ReviewWorld, id: PlayerId) {
    let ferry = l
        .mechanics
        .iter()
        .find(|m| m.kind == MechanicKind::MovingPlatform)
        .unwrap();
    // Leave the conveyor/ice fully before waiting, rather than neutral-drifting
    // away from the boarding mark while the ferry returns.
    walk_to(l, w, id, ferry.rect.x - 3 * UNIT / 4);
    // Board from the left while the shuttle is outbound. The approach leaves
    // enough horizontal clearance to rise above the solid platform's side.
    let mut boarded = false;
    for _ in 0..800 {
        let state = w.objects.iter().find(|o| o.id == ferry.id).unwrap();
        let p = w.players.iter().find(|p| p.id == id).unwrap();
        if state.timer < ferry.period / 2
            && state.rect.x >= ferry.rect.x + 3 * UNIT / 4
            && state.rect.x <= ferry.rect.x + 17 * UNIT / 20
            && (p.x - (ferry.rect.x - 3 * UNIT / 4)).abs() <= 96
            && p.vx.abs() <= 8
        {
            run(l, w, id, 100, true);
            boarded = true;
            break;
        }
        let input = steering(l, p, ferry.rect.x - 3 * UNIT / 4);
        w.step(l, &[(id, input)]);
    }
    assert!(boarded, "ferry {} never reached boarding window", l.number);
    assert_eq!(w.deaths, 0, "ferry {}: {:?}", l.number, w.players);
    let hazard = l
        .mechanics
        .iter()
        .find(|m| m.kind == MechanicKind::Hazard)
        .unwrap();
    assert!(w.players.iter().find(|p| p.id == id).unwrap().x >= hazard.rect.x + hazard.rect.w);
}

#[test]
fn input_only_completion_of_ferry_and_cold_machinery() {
    for n in [7, 17] {
        let l = level(n).unwrap();
        let mut w = ReviewWorld::new(&l);
        cross_ferry(&l, &mut w, 2);
        walk_to(&l, &mut w, 2, l.exit.x + UNIT);
        cross_ferry(&l, &mut w, 1);
        walk_to(&l, &mut w, 1, l.exit.x);
        hold(&l, &mut w, 30);
        assert!(w.completed, "ferry {n}: {:?}", w.players);
        assert_eq!(w.deaths, 0);
    }
}

#[test]
fn input_only_completion_of_finale_with_shared_lift() {
    let l = level(20).unwrap();
    let mut w = ReviewWorld::new(&l);
    cross_ferry(&l, &mut w, 2);
    // First partner waits against the final key wall, beyond the timed door.
    run(&l, &mut w, 2, 300, false);
    assert_eq!(w.players[1].x + PLAYER_WIDTH, l.gate.x);
    cross_ferry(&l, &mut w, 1);
    walk_to(&l, &mut w, 1, l.gate.x - 17 * UNIT / 8);
    run(&l, &mut w, 1, 50, true);
    hold(&l, &mut w, 1);
    run(&l, &mut w, 1, 60, true);
    assert!(w.key_collected, "final lift: {:?}", w.players);
    for _ in 0..300 {
        let commands: Vec<_> = w
            .players
            .iter()
            .map(|p| {
                (
                    p.id,
                    Input {
                        axis: i8::from(p.x < l.exit.x + UNIT),
                        jump: false,
                    },
                )
            })
            .collect();
        w.step(&l, &commands);
        if w.completed {
            break;
        }
    }
    assert!(w.completed, "final exit: {:?}", w.players);
    assert_eq!(w.deaths, 0);
}
