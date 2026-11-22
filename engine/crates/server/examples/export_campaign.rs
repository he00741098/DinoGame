//! Export test-only wire fixtures from the actual authoritative catalog.
//! No files/assets from the GPL development host are linked into the native app.
use net_proto::{ServerMessage, PROTOCOL_VERSION, RULESET};
use sim_core::{campaign, World, TICK_HZ};

fn main() {
    let first = campaign::level(1).unwrap();
    println!(
        "{}",
        serde_json::to_string(&ServerMessage::Welcome {
            protocol: PROTOCOL_VERSION,
            ruleset: RULESET.into(),
            player: 1,
            tick_hz: TICK_HZ,
            snapshot_hz: 20,
            level: first,
            resume_token: "0".repeat(64),
            resumed: false,
            revision: 1,
            controller: Some(1),
        })
        .unwrap()
    );
    for number in 1..=campaign::LEVEL_COUNT {
        let level = campaign::level(number).unwrap();
        let mut world = World::for_level(&level);
        world.tick = u64::from(number) * 1000;
        world.level_start_tick = world.tick;
        for id in 1..=4 {
            assert!(world.add_player_in(&level, id));
        }
        for _ in 0..60 {
            world.step(&level, &[]);
        }
        println!(
            "{}",
            serde_json::to_string(&ServerMessage::Snapshot {
                world,
                acks: vec![],
                level,
                revision: u32::from(number),
                controller: Some(1),
                campaign_complete: false,
            })
            .unwrap()
        );
    }
}
