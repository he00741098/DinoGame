use super::*;

fn join(room: &mut Room, now: Instant) -> Joined {
    room.join(
        watch::channel(String::new()).0,
        watch::channel(false).0,
        None,
        now,
    )
    .unwrap()
}

#[test]
fn automatic_advance_waits_for_celebration_and_rebaselines_every_player() {
    let mut room = Room::new();
    let now = Instant::now();
    let owner = join(&mut room, now);
    let peer = join(&mut room, now);
    room.world.completed = true; // Timing/control test; solvability tested input-only.
    let level = room.level.clone();
    room.tick(&level, now);
    room.tick(&level, now + AUTO_ADVANCE_DELAY - Duration::from_millis(1));
    assert_eq!(room.level.number, 1);
    room.tick(&level, now + AUTO_ADVANCE_DELAY);
    assert_eq!(room.level.number, 2);
    assert_eq!(room.revision, 2);
    assert!(!room.world.completed);
    for id in [owner.player, peer.player] {
        assert!(room.slots[&id].needs_jump_baseline);
        assert!(room.slots[&id].ack.is_none());
        assert_eq!(room.slots[&id].held, Input::default());
    }
    assert!(room.completed_at.is_none());
}

#[test]
fn auto_advance_pauses_for_disconnection_and_never_wraps_finale() {
    let mut room = Room::new();
    let now = Instant::now();
    join(&mut room, now);
    let peer = join(&mut room, now);
    room.world.completed = true;
    let level = room.level.clone();
    room.tick(&level, now);
    room.leave(peer.player, peer.generation, false, now);
    room.tick(&level, now + AUTO_ADVANCE_DELAY);
    assert_eq!(room.level.number, 1);
    assert!(room.completed_at.is_none());
    room.slots.get_mut(&peer.player).unwrap().connected = true;
    room.tick(&level, now + AUTO_ADVANCE_DELAY);
    assert_eq!(room.level.number, 1);
    room.level = campaign::level(campaign::LEVEL_COUNT).unwrap();
    room.world = World::for_level(&room.level);
    room.world.completed = true;
    let finale = room.level.clone();
    room.tick(&finale, now + AUTO_ADVANCE_DELAY * 3);
    assert_eq!(room.level.number, campaign::LEVEL_COUNT);
    assert!(room.world.completed);
    assert!(room.completed_at.is_none());
}

#[test]
fn completed_campaign_suppresses_unchanged_publications_but_not_acks_or_restart() {
    let mut room = Room::new();
    let now = Instant::now();
    let owner = join(&mut room, now);
    room.level = campaign::level(campaign::LEVEL_COUNT).unwrap();
    room.world = World::for_level(&room.level);
    room.world.completed = true;
    assert!(room.publication().is_some());
    room.world.tick += 60;
    assert!(room.publication().is_none());
    room.slots.get_mut(&owner.player).unwrap().ack = Some(1);
    assert!(room.publication().is_some());
    assert!(room.publication().is_none());
    assert!(room.control(owner.player, owner.generation, 2, 1, false));
    assert!(room.publication().is_some());
}

#[test]
fn restarting_during_celebration_cancels_automatic_advance() {
    let mut room = Room::new();
    let now = Instant::now();
    let owner = join(&mut room, now);
    join(&mut room, now);
    room.world.completed = true;
    let level = room.level.clone();
    room.tick(&level, now);
    assert!(room.control(owner.player, owner.generation, 1, 1, false));
    assert!(room.completed_at.is_none());
    room.tick(&level, now + AUTO_ADVANCE_DELAY * 2);
    assert_eq!(room.level.number, 1);
    assert_eq!(room.revision, 2);
    assert!(!room.world.completed);
}

#[test]
fn campaign_controls_are_authorized_atomic_and_bounded_at_twenty() {
    let mut room = Room::new();
    let now = Instant::now();
    let owner = join(&mut room, now);
    let peer = join(&mut room, now);
    assert_eq!(room.level.number, 1);
    assert!(!room.control(peer.player, peer.generation, 1, 1, false));
    assert!(!room.control(owner.player, owner.generation, 1, 1, true));
    assert_eq!(room.revision, 1);
    room.world.tick = 100;
    for number in 1..campaign::LEVEL_COUNT {
        room.world.completed = true; // Transport/control test, not a solvability claim.
        let revision = room.revision;
        assert!(room.control(
            owner.player,
            owner.generation,
            u32::from(number),
            revision,
            true
        ));
        assert_eq!(room.level.number, number + 1);
        assert_eq!(room.revision, revision + 1);
        assert_eq!(room.world.tick, 100);
        assert_eq!(room.world.level_start_tick, 100);
        assert_eq!(
            room.world.players.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![owner.player, peer.player]
        );
        assert!(!room.world.completed);
        assert!(!room.control(owner.player, owner.generation, 1000, revision, true));
    }
    room.world.completed = true;
    let revision = room.revision;
    assert!(!room.control(owner.player, owner.generation, 1000, revision, true));
    assert_eq!(room.level.number, campaign::LEVEL_COUNT);
    let ServerMessage::Snapshot {
        campaign_complete, ..
    } = room.snapshot()
    else {
        panic!("snapshot")
    };
    assert!(campaign_complete);
    assert!(room.control(owner.player, owner.generation, 1001, revision, false));
    assert_eq!(room.level.number, campaign::LEVEL_COUNT);
    assert!(!room.world.completed);
}

#[test]
fn revision_fences_old_input_and_rebaselines_jump_ids_without_ghost_actions() {
    let mut room = Room::new();
    let now = Instant::now();
    let owner = join(&mut room, now);
    assert!(room.input(owner.player, owner.generation, 1, Input::default(), 5, now));
    assert!(room.control(owner.player, owner.generation, 2, 1, false));
    assert!(!room.command(
        owner.player,
        owner.generation,
        ClientMessage::Input {
            seq: 3,
            revision: 1,
            jump_id: 6,
            input: Input {
                axis: 1,
                jump: true
            },
        },
        now
    ));
    assert_eq!(room.slots[&owner.player].latest_seq, 2);
    assert!(room.command(
        owner.player,
        owner.generation,
        ClientMessage::Input {
            seq: 4,
            revision: 2,
            jump_id: 6,
            input: Input {
                axis: 1,
                jump: true
            },
        },
        now
    ));
    let level = room.level.clone();
    room.tick(&level, now);
    assert_eq!(room.world.players[0].vx, 0);
    assert!(!room.world.players[0].jump_held);
    assert!(room.command(
        owner.player,
        owner.generation,
        ClientMessage::Input {
            seq: 5,
            revision: 2,
            jump_id: 7,
            input: Input {
                axis: 0,
                jump: true
            },
        },
        now
    ));
    room.tick(&level, now);
    assert!(room.world.players[0].vy < 0);
}

#[test]
fn controller_handoff_and_resume_preserve_current_level_not_a_stale_welcome() {
    let mut room = Room::new();
    let now = Instant::now();
    let first = join(&mut room, now);
    let second = join(&mut room, now);
    room.world.completed = true;
    assert!(room.control(first.player, first.generation, 1, 1, true));
    room.leave(first.player, first.generation, false, now);
    assert_eq!(room.controller(), Some(second.player));
    assert!(room.control(second.player, second.generation, 1, 2, false));
    let resumed = room
        .join(
            watch::channel(String::new()).0,
            watch::channel(false).0,
            Some(&first.token),
            now,
        )
        .unwrap();
    let ServerMessage::Snapshot {
        level,
        revision,
        controller,
        ..
    } = resumed.snapshot
    else {
        panic!("snapshot")
    };
    assert_eq!(level.number, 2);
    assert_eq!(revision, 3);
    assert_eq!(controller, Some(first.player));
    assert!(!room.control(first.player, first.generation, 99, 3, true));
}

#[test]
fn every_campaign_snapshot_fits_the_native_receive_limit() {
    for number in 1..=campaign::LEVEL_COUNT {
        let mut room = Room::new();
        room.level = campaign::level(number).unwrap();
        room.world = World::for_level(&room.level);
        for _ in 0..MAX_PLAYERS {
            join(&mut room, Instant::now());
        }
        let encoded = serde_json::to_vec(&room.snapshot()).unwrap();
        assert!(
            encoded.len() <= 65_536,
            "level {number}: {} bytes",
            encoded.len()
        );
    }
}

#[tokio::test(start_paused = true)]
async fn empty_rooms_keep_bounded_stage_progress_without_retaining_player_credentials() {
    let host = Host::default();
    let joined = host
        .join(
            "progress",
            watch::channel(String::new()).0,
            watch::channel(false).0,
            None,
        )
        .await
        .unwrap();
    {
        let mut rooms = host.rooms.lock().await;
        let room = rooms.get_mut("progress").unwrap();
        room.world.completed = true;
        assert!(room.control(joined.player, joined.generation, 1, 1, true));
    }
    host.leave("progress", joined.player, joined.generation, true)
        .await;
    assert!(host.rooms.lock().await.is_empty());
    assert!(matches!(
        host.join(
            "progress",
            watch::channel(String::new()).0,
            watch::channel(false).0,
            Some(&joined.token)
        )
        .await,
        Err("invalid_resume_token")
    ));
    let fresh = host
        .join(
            "progress",
            watch::channel(String::new()).0,
            watch::channel(false).0,
            None,
        )
        .await
        .unwrap();
    let ServerMessage::Snapshot { level, world, .. } = fresh.snapshot else {
        panic!("snapshot")
    };
    assert_eq!(level.number, 2);
    assert!(!world.completed);
    assert_ne!(fresh.token, joined.token);
    host.leave("progress", fresh.player, fresh.generation, true)
        .await;
    tokio::time::advance(PROGRESS_TTL).await;
    let expired = host
        .join(
            "progress",
            watch::channel(String::new()).0,
            watch::channel(false).0,
            None,
        )
        .await
        .unwrap();
    let ServerMessage::Snapshot { level, .. } = expired.snapshot else {
        panic!("snapshot")
    };
    assert_eq!(level.number, 1);
}

#[test]
fn saved_stage_cache_has_a_hard_memory_bound_and_expiry() {
    let mut cache = BTreeMap::new();
    let room = Room::new();
    let now = Instant::now();
    for index in 0..1000 {
        save_progress(
            &mut cache,
            &format!("room-{index}"),
            &room,
            now + Duration::from_millis(index),
        );
        assert!(cache.len() <= MAX_SAVED_ROOMS);
    }
    save_progress(
        &mut cache,
        "later",
        &room,
        now + PROGRESS_TTL + Duration::from_secs(10),
    );
    assert_eq!(cache.len(), 1);
}
