use sim_core::{Level, Mechanic, MechanicKind, Rect, World, PLAYER_HEIGHT, UNIT};

#[test]
fn uninitialized_world_checks_dynamic_surfaces_before_spawning() {
    let mut level = Level::default();
    let obstruction = Rect {
        x: 0,
        y: 11 * UNIT,
        w: 8 * UNIT,
        h: UNIT,
    };
    level.mechanics.push(Mechanic {
        id: 1,
        kind: MechanicKind::Crumble,
        rect: obstruction,
        travel: 0,
        period: 70,
    });
    assert!(level.valid());
    let mut world = World::new();
    assert!(world.add_player_in(&level, 1));
    assert!(!world.players[0].rect().overlaps(obstruction));
    assert_eq!(world.objects.len(), 1);
}

#[test]
fn checkpoint_above_player_height_falls_back_to_safe_start() {
    let mut level = Level::default();
    let checkpoint = Rect {
        x: UNIT,
        y: 0,
        w: 4 * UNIT,
        h: 1,
    };
    level.mechanics.push(Mechanic {
        id: 1,
        kind: MechanicKind::Checkpoint,
        rect: checkpoint,
        travel: 0,
        period: 0,
    });
    assert!(level.valid());
    let mut world = World::for_level(&level);
    world.checkpoint = Some(checkpoint);
    assert!(world.add_player_in(&level, 1));
    assert_eq!(world.players[0].y, 12 * UNIT - PLAYER_HEIGHT);
    assert!(world.checkpoint.is_none());
}
