use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use multiplayer_server::{app, Host};
use net_proto::{ClientMessage, ServerMessage, GAME_ID, PROTOCOL_VERSION, RULESET};
use sim_core::{Input, World, PLAYER_HEIGHT, PLAYER_WIDTH, UNIT};
use tokio::{net::TcpStream, task::JoinHandle, time::timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn drive_pair_until(
    a: &mut Socket,
    b: &mut Socket,
    sequence: &mut u32,
    jumps: [u32; 2],
    commands: impl Fn(&World) -> [Input; 2],
    reached: impl Fn(&World) -> bool,
) -> World {
    let mut latest = loop {
        let first = receive(a).await;
        let _ = receive(b).await;
        if let ServerMessage::Snapshot { world, .. } = first {
            break world;
        }
    };
    let deadline = latest.tick + 600;
    loop {
        if reached(&latest) {
            return latest;
        }
        assert!(
            latest.tick < deadline,
            "co-op drive timed out at {latest:?}"
        );
        let inputs = commands(&latest);
        *sequence += 1;
        for (socket, input, jump_id) in [
            (&mut *a, inputs[0], jumps[0]),
            (&mut *b, inputs[1], jumps[1]),
        ] {
            send(
                socket,
                ClientMessage::Input {
                    seq: *sequence,
                    revision: 1,
                    input,
                    jump_id,
                },
            )
            .await;
        }
        let first = receive(a).await;
        let _ = receive(b).await;
        if let ServerMessage::Snapshot { world, .. } = first {
            latest = world;
        }
    }
}

#[tokio::test]
async fn compact_writer_uses_exact_bases_and_full_frames_after_restart() {
    timeout(Duration::from_secs(5), async {
        let host = TestHost::start().await;
        let mut socket = connect_async(&host.url).await.unwrap().0;
        send(
            &mut socket,
            ClientMessage::Hello {
                protocol: PROTOCOL_VERSION,
                game: GAME_ID.into(),
                ruleset: RULESET.into(),
                room: "patch-test".into(),
                resume_token: None,
                patches: true,
            },
        )
        .await;
        assert_eq!(welcome(&mut socket).await, 1);
        let mut tick = None;
        let mut saw_patch = false;
        send(
            &mut socket,
            ClientMessage::Input {
                seq: 1,
                revision: 1,
                jump_id: 0,
                input: Input::default(),
            },
        )
        .await;
        for _ in 0..20 {
            let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                continue;
            };
            let message: serde_json::Value = serde_json::from_str(&text).unwrap();
            if message["type"] == "patch" {
                assert_eq!(message["base_tick"].as_u64(), tick);
                assert!(message["changes"].get("level").is_none());
                tick = message["changes"]["world"]["tick"].as_u64().or(tick);
                saw_patch = true;
                if message["changes"]["acks"]
                    .as_array()
                    .is_some_and(|acks| acks.iter().any(|ack| ack["seq"] == 1))
                {
                    break;
                }
            } else {
                assert_eq!(message["type"], "snapshot");
                tick = message["world"]["tick"].as_u64();
            }
        }
        assert!(saw_patch);
        send(
            &mut socket,
            ClientMessage::Restart {
                seq: 2,
                revision: 1,
            },
        )
        .await;
        loop {
            let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                continue;
            };
            let message: serde_json::Value = serde_json::from_str(&text).unwrap();
            if message["revision"] == 2 {
                assert_eq!(message["type"], "snapshot");
                assert!(message.get("level").is_some());
                break;
            }
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn input_only_coop_completion_auto_advances_both_real_sockets_to_level_two() {
    timeout(Duration::from_secs(30), async {
        let host = TestHost::start().await;
        let mut a = host.connect("next-flow").await;
        assert_eq!(welcome(&mut a).await, 1);
        let mut b = host.connect("next-flow").await;
        assert_eq!(welcome(&mut b).await, 2);
        let neutral = Input::default();
        let right = Input {
            axis: 1,
            jump: false,
        };
        let jump_right = Input {
            axis: 1,
            jump: true,
        };
        let mut sequence = 0;
        // Lower player walks to the barrier, upper player approaches and lands
        // on the lower player's shoulders. No state or positions are injected.
        drive_pair_until(
            &mut a,
            &mut b,
            &mut sequence,
            [0, 0],
            |_| [neutral, right],
            |world| world.players.len() == 2 && world.players[1].x + PLAYER_WIDTH == 9 * UNIT,
        )
        .await;
        drive_pair_until(
            &mut a,
            &mut b,
            &mut sequence,
            [0, 0],
            |_| [right, neutral],
            |world| world.players[0].x >= 7 * UNIT - 128,
        )
        .await;
        drive_pair_until(
            &mut a,
            &mut b,
            &mut sequence,
            [1, 0],
            |_| [jump_right, neutral],
            |world| {
                world.players[0].grounded
                    && world.players[0].y + PLAYER_HEIGHT == world.players[1].y
            },
        )
        .await;
        drive_pair_until(
            &mut a,
            &mut b,
            &mut sequence,
            [2, 0],
            |_| [jump_right, neutral],
            |world| world.key_collected,
        )
        .await;
        let won = drive_pair_until(
            &mut a,
            &mut b,
            &mut sequence,
            [2, 0],
            |world| {
                std::array::from_fn(|i| Input {
                    // Both players stop at the doorway lip, rather than needing
                    // the first arrival to shuffle deeper to make physical room.
                    axis: if world.players[i].x < 22 * UNIT - PLAYER_WIDTH + 1 {
                        1
                    } else {
                        0
                    },
                    jump: false,
                })
            },
            |world| world.completed,
        )
        .await;
        assert_eq!(won.deaths, 0);
        assert!(won.players.iter().all(|p| p.x < 22 * UNIT));
        // No Next command: the authority advances after its short celebration.
        for socket in [&mut a, &mut b] {
            loop {
                if let ServerMessage::Snapshot {
                    world,
                    level,
                    revision,
                    controller,
                    ..
                } = receive(socket).await
                {
                    if revision == 2 {
                        assert_eq!(level.number, 2);
                        assert_eq!(controller, Some(1));
                        assert!(!world.completed);
                        assert!(world.tick >= won.tick);
                        assert_eq!(
                            world.players.iter().map(|p| p.id).collect::<Vec<_>>(),
                            vec![1, 2]
                        );
                        break;
                    }
                }
            }
        }
    })
    .await
    .expect("live campaign completion exceeded its wall-clock bound");
}

struct TestHost {
    url: String,
    http: JoinHandle<()>,
    ticks: JoinHandle<()>,
}

impl TestHost {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/ws", listener.local_addr().unwrap());
        let host = Host::default();
        let ticker = host.clone();
        Self {
            url,
            http: tokio::spawn(async move {
                axum::serve(listener, app(host)).await.unwrap();
            }),
            ticks: tokio::spawn(async move {
                ticker.run_ticks().await;
            }),
        }
    }

    async fn connect(&self, room: &str) -> Socket {
        let mut socket = connect_async(&self.url).await.unwrap().0;
        send(
            &mut socket,
            ClientMessage::Hello {
                protocol: PROTOCOL_VERSION,
                game: GAME_ID.into(),
                ruleset: RULESET.into(),
                room: room.into(),
                resume_token: None,
                patches: false,
            },
        )
        .await;
        socket
    }
}

impl Drop for TestHost {
    fn drop(&mut self) {
        self.http.abort();
        self.ticks.abort();
    }
}

async fn send(socket: &mut Socket, message: ClientMessage) {
    socket
        .send(Message::Text(
            serde_json::to_string(&message).unwrap().into(),
        ))
        .await
        .unwrap();
}

async fn receive(socket: &mut Socket) -> ServerMessage {
    timeout(Duration::from_secs(3), async {
        loop {
            let message = socket
                .next()
                .await
                .expect("socket closed")
                .expect("socket error");
            if let Message::Text(text) = message {
                return serde_json::from_str(&text).unwrap();
            }
        }
    })
    .await
    .expect("server timed out")
}

async fn welcome(socket: &mut Socket) -> u32 {
    match receive(socket).await {
        ServerMessage::Welcome {
            player, protocol, ..
        } => {
            assert_eq!(protocol, PROTOCOL_VERSION);
            player
        }
        other => panic!("expected welcome, got {other:?}"),
    }
}

#[tokio::test]
async fn two_real_clients_receive_authority_and_acknowledged_input() {
    let host = TestHost::start().await;
    let mut a = host.connect("pair").await;
    let id = welcome(&mut a).await;
    let mut b = host.connect("pair").await;
    assert_ne!(welcome(&mut b).await, id);
    send(
        &mut a,
        ClientMessage::Input {
            seq: 10,
            revision: 1,
            jump_id: 0,
            input: Input {
                axis: -1,
                jump: false,
            },
        },
    )
    .await;
    for _ in 0..20 {
        if let ServerMessage::Snapshot { world, acks, .. } = receive(&mut b).await {
            if acks.iter().any(|a| a.player == id && a.seq == 10) {
                assert_eq!(world.players.len(), 2);
                assert!(world.players.iter().find(|p| p.id == id).unwrap().x < 2 * sim_core::UNIT);
                a.close(None).await.unwrap();
                return;
            }
        }
    }
    panic!("input was never acknowledged");
}

#[tokio::test]
async fn incompatible_protocol_and_ruleset_are_rejected_before_join() {
    let host = TestHost::start().await;
    for (protocol, ruleset, expected) in [
        (999, RULESET, "protocol_mismatch"),
        (PROTOCOL_VERSION, "other-level", "ruleset_mismatch"),
    ] {
        let mut socket = connect_async(&host.url).await.unwrap().0;
        send(
            &mut socket,
            ClientMessage::Hello {
                protocol,
                game: GAME_ID.into(),
                ruleset: ruleset.into(),
                room: "pair".into(),
                resume_token: None,
                patches: false,
            },
        )
        .await;
        let ServerMessage::Error { code, .. } = receive(&mut socket).await else {
            panic!("expected error")
        };
        assert_eq!(code, expected);
    }
    let mut valid = host.connect("pair").await;
    assert_eq!(welcome(&mut valid).await, 1);
}

#[tokio::test]
async fn malformed_peer_is_removed_without_affecting_other_clients() {
    let host = TestHost::start().await;
    let mut bad = host.connect("pair").await;
    let id = welcome(&mut bad).await;
    let _ = receive(&mut bad).await;
    let mut good = host.connect("pair").await;
    welcome(&mut good).await;
    bad.send(Message::Text("not json".into())).await.unwrap();
    let ServerMessage::Error { code, .. } = receive(&mut bad).await else {
        panic!("expected error")
    };
    assert_eq!(code, "invalid_message");
    for _ in 0..20 {
        if let ServerMessage::Snapshot { world, .. } = receive(&mut good).await {
            if !world.players.iter().any(|p| p.id == id) {
                assert_eq!(world.players.len(), 1);
                return;
            }
        }
    }
    panic!("malformed player not removed");
}

#[tokio::test]
async fn rooms_do_not_leak_players_and_fifth_player_is_rejected() {
    let host = TestHost::start().await;
    let mut peers = Vec::new();
    for _ in 0..4 {
        let mut socket = host.connect("full").await;
        welcome(&mut socket).await;
        peers.push(socket);
    }
    let mut fifth = host.connect("full").await;
    let ServerMessage::Error { code, .. } = receive(&mut fifth).await else {
        panic!("expected error")
    };
    assert_eq!(code, "room_full");
    let mut separate = host.connect("separate").await;
    assert_eq!(welcome(&mut separate).await, 1);
    let ServerMessage::Snapshot { world, .. } = receive(&mut separate).await else {
        panic!("expected snapshot")
    };
    assert_eq!(world.players.len(), 1);
}

#[tokio::test]
async fn clean_disconnect_removes_player_from_next_snapshots() {
    let host = TestHost::start().await;
    let mut a = host.connect("pair").await;
    let id = welcome(&mut a).await;
    let mut b = host.connect("pair").await;
    welcome(&mut b).await;
    a.close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
        reason: "".into(),
    }))
    .await
    .unwrap();
    for _ in 0..20 {
        if let ServerMessage::Snapshot { world, .. } = receive(&mut b).await {
            if !world.players.iter().any(|p| p.id == id) {
                return;
            }
        }
    }
    panic!("disconnected player remains in room");
}

async fn resume(host: &TestHost, room: &str, token: &str) -> Socket {
    let mut socket = connect_async(&host.url).await.unwrap().0;
    send(
        &mut socket,
        ClientMessage::Hello {
            protocol: PROTOCOL_VERSION,
            game: GAME_ID.into(),
            ruleset: RULESET.into(),
            room: room.into(),
            resume_token: Some(token.into()),
            patches: false,
        },
    )
    .await;
    socket
}

#[tokio::test]
async fn oversized_websocket_frame_permanently_removes_session_and_token() {
    let host = TestHost::start().await;
    let mut oversized = host.connect("room").await;
    let ServerMessage::Welcome {
        player,
        resume_token,
        ..
    } = receive(&mut oversized).await
    else {
        panic!("welcome")
    };
    let mut observer = host.connect("room").await;
    welcome(&mut observer).await;
    // Only 4097 bytes: enough to exercise tungstenite's frame capacity error, not JSON decoding.
    oversized
        .send(Message::Text(
            "x".repeat(net_proto::MAX_MESSAGE_BYTES + 1).into(),
        ))
        .await
        .unwrap();
    for _ in 0..20 {
        if let ServerMessage::Snapshot { world, .. } = receive(&mut observer).await {
            if !world.players.iter().any(|p| p.id == player) {
                let mut attempted = resume(&host, "room", &resume_token).await;
                let ServerMessage::Error { code, .. } = receive(&mut attempted).await else {
                    panic!("expected invalid token")
                };
                assert_eq!(code, "invalid_resume_token");
                return;
            }
        }
    }
    panic!("oversized frame retained a resumable session");
}

#[tokio::test]
async fn active_full_room_resume_preserves_identity_resets_acks_and_stops_old_socket() {
    let host = TestHost::start().await;
    let mut old = host.connect("full").await;
    let ServerMessage::Welcome {
        player,
        resume_token,
        level,
        resumed,
        ..
    } = receive(&mut old).await
    else {
        panic!("welcome")
    };
    assert!(!resumed);
    assert_eq!(resume_token.len(), 64);
    send(
        &mut old,
        ClientMessage::Input {
            seq: 99,
            revision: 1,
            jump_id: 1,
            input: Input {
                axis: 1,
                jump: true,
            },
        },
    )
    .await;
    loop {
        if let ServerMessage::Snapshot { acks, .. } = receive(&mut old).await {
            if acks.iter().any(|ack| ack.player == player && ack.seq == 99) {
                break;
            }
        }
    }
    let mut others = Vec::new();
    for _ in 1..4 {
        let mut peer = host.connect("full").await;
        welcome(&mut peer).await;
        others.push(peer);
    }
    let mut replacement = resume(&host, "full", &resume_token).await;
    let ServerMessage::Welcome {
        player: new_player,
        resumed,
        level: new_level,
        resume_token: new_token,
        ..
    } = receive(&mut replacement).await
    else {
        panic!("welcome")
    };
    assert!(resumed);
    assert_eq!(new_player, player);
    assert_eq!(level, new_level);
    assert_eq!(new_token, resume_token);
    let ServerMessage::Snapshot { world, acks, .. } = receive(&mut replacement).await else {
        panic!("snapshot")
    };
    assert_eq!(world.players.len(), 4);
    assert!(!acks.iter().any(|ack| ack.player == player));
    assert_eq!(world.players.iter().find(|p| p.id == player).unwrap().vx, 0);
    timeout(Duration::from_secs(2), async {
        loop {
            match old.next().await {
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {}
            }
        }
    })
    .await
    .expect("replaced socket not stopped");
    send(
        &mut replacement,
        ClientMessage::Input {
            seq: 1,
            revision: 1,
            jump_id: 0,
            input: Input::default(),
        },
    )
    .await;
    loop {
        if let ServerMessage::Snapshot { acks, .. } = receive(&mut replacement).await {
            assert!(!acks.iter().any(|ack| ack.player == player && ack.seq == 99));
            if acks.iter().any(|ack| ack.player == player && ack.seq == 1) {
                break;
            }
        }
    }
}

#[tokio::test]
async fn abnormal_disconnect_resumes_but_wrong_room_or_token_is_rejected() {
    let host = TestHost::start().await;
    let mut old = host.connect("room").await;
    let ServerMessage::Welcome {
        player,
        resume_token,
        ..
    } = receive(&mut old).await
    else {
        panic!("welcome")
    };
    drop(old); // No close frame: retain for grace.
    tokio::time::sleep(Duration::from_millis(20)).await;
    for (room, token) in [("room", "bad"), ("other", resume_token.as_str())] {
        let mut bad = resume(&host, room, token).await;
        let ServerMessage::Error { code, .. } = receive(&mut bad).await else {
            panic!("error")
        };
        assert_eq!(code, "invalid_resume_token");
    }
    let mut replacement = resume(&host, "room", &resume_token).await;
    let ServerMessage::Welcome {
        player: same_player,
        resumed,
        ..
    } = receive(&mut replacement).await
    else {
        panic!("welcome")
    };
    assert!(resumed);
    assert_eq!(same_player, player);
    replacement
        .close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
            code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Away,
            reason: "network switching".into(),
        }))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let mut replacement = resume(&host, "room", &resume_token).await;
    let ServerMessage::Welcome {
        player: same_player,
        resumed,
        ..
    } = receive(&mut replacement).await
    else {
        panic!("welcome after 1001")
    };
    assert!(resumed);
    assert_eq!(same_player, player);
    replacement
        .close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
            code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
            reason: "".into(),
        }))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let mut gone = resume(&host, "room", &resume_token).await;
    let ServerMessage::Error { code, .. } = receive(&mut gone).await else {
        panic!("error")
    };
    assert_eq!(code, "invalid_resume_token");
}
