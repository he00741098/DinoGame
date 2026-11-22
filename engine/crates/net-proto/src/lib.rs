//! Versioned development protocol. Positions and time are never client commands.
use serde::{Deserialize, Serialize};
use sim_core::{Input, Level, PlayerId, World};

pub const PROTOCOL_VERSION: u32 = 3;
pub const GAME_ID: &str = "coop-sandbox";
pub const RULESET: &str = "coop-campaign-3";
pub const MAX_MESSAGE_BYTES: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    Hello {
        protocol: u32,
        game: String,
        ruleset: String,
        room: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resume_token: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        patches: bool,
    },
    Input {
        seq: u32,
        input: Input,
        jump_id: u32,
        revision: u32,
    },
    Restart {
        seq: u32,
        revision: u32,
    },
    Next {
        seq: u32,
        revision: u32,
    },
    /// Preset only: the server owns the wording, so clients cannot send free text.
    Chat {
        preset: u8,
    },
}

/// Canonical quick-chat wording. Index is the wire `preset` value.
pub const CHAT_PRESETS: [&str; 6] = [
    "Follow me",
    "Wait here",
    "Jump!",
    "Ready?",
    "Help!",
    "Nice!",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputAck {
    pub player: PlayerId,
    pub seq: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        protocol: u32,
        ruleset: String,
        player: PlayerId,
        tick_hz: u32,
        snapshot_hz: u32,
        level: Level,
        resume_token: String,
        resumed: bool,
        revision: u32,
        controller: Option<PlayerId>,
    },
    Snapshot {
        world: World,
        acks: Vec<InputAck>,
        level: Level,
        revision: u32,
        controller: Option<PlayerId>,
        campaign_complete: bool,
    },
    Error {
        code: String,
        message: String,
    },
    Chat {
        player: PlayerId,
        preset: u8,
        text: String,
    },
}

pub fn valid_room(room: &str) -> bool {
    (3..=16).contains(&room.len()) && room.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

pub fn decode(text: &str) -> Result<ClientMessage, String> {
    if text.len() > MAX_MESSAGE_BYTES {
        return Err("message too large".into());
    }
    let message: ClientMessage = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if let ClientMessage::Input { input, .. } = message {
        if !input.valid() {
            return Err("axis must be -1, 0, or 1".into());
        }
    }
    Ok(message)
}

pub fn validate_hello(message: &ClientMessage) -> Result<&str, &'static str> {
    match message {
        ClientMessage::Hello {
            protocol,
            game,
            ruleset,
            room,
            ..
        } => {
            if *protocol != PROTOCOL_VERSION {
                return Err("protocol_mismatch");
            }
            if game != GAME_ID || ruleset != RULESET {
                return Err("ruleset_mismatch");
            }
            if !valid_room(room) {
                return Err("invalid_room");
            }
            Ok(room)
        }
        _ => Err("hello_required"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_inputs_and_client_positions() {
        for text in [
            r#"{"type":"input","seq":1,"revision":1,"jump_id":0,"input":{"axis":2,"jump":false}}"#,
            r#"{"type":"input","seq":1,"revision":1,"jump_id":0,"input":{"axis":0,"jump":false},"x":100}"#,
            r#"{"type":"input","seq":1,"revision":1,"jump_id":0,"input":{"axis":0,"jump":false,"x":100}}"#,
            r#"{"type":"input","seq":1,"revision":1,"input":{"axis":0,"jump":false}}"#,
            r#"{"type":"input","seq":1,"jump_id":0,"input":{"axis":0,"jump":false}}"#,
            r#"{"type":"position","x":100}"#,
            "not json",
        ] {
            assert!(decode(text).is_err(), "{text}");
        }
    }

    #[test]
    fn enforces_version_ruleset_and_room() {
        let mut hello = ClientMessage::Hello {
            protocol: PROTOCOL_VERSION,
            game: GAME_ID.into(),
            ruleset: RULESET.into(),
            room: "test-1".into(),
            resume_token: None,
            patches: false,
        };
        assert_eq!(validate_hello(&hello), Ok("test-1"));
        if let ClientMessage::Hello { protocol, .. } = &mut hello {
            *protocol += 1;
        }
        assert_eq!(validate_hello(&hello), Err("protocol_mismatch"));
        assert!(!valid_room("../../secret"));
        assert!(!valid_room("ab"));
        assert!(decode(&"x".repeat(MAX_MESSAGE_BYTES + 1)).is_err());
    }

    #[test]
    fn v3_optional_resume_and_required_jump_and_revision_identifiers() {
        for extra in ["", r#", "resume_token":"opaque""#] {
            let hello = decode(&format!(r#"{{"type":"hello","protocol":3,"game":"coop-sandbox","ruleset":"coop-campaign-3","room":"test"{extra}}}"#)).unwrap();
            assert_eq!(validate_hello(&hello), Ok("test"));
        }
        assert!(decode(
            r#"{"type":"input","seq":1,"revision":1,"jump_id":0,"input":{"axis":0,"jump":false}}"#
        )
        .is_ok());
        assert!(decode(r#"{"type":"hello","protocol":1,"game":"coop-sandbox","ruleset":"coop-sandbox-1","room":"test"}"#).is_ok_and(|h| validate_hello(&h) == Err("protocol_mismatch")));
    }

    #[test]
    fn snapshot_round_trip_preserves_hash() {
        let mut world = World::new();
        world.add_player(1);
        world.step(&Level::default(), &[]);
        let hash = world.state_hash();
        let encoded = serde_json::to_string(&ServerMessage::Snapshot {
            world,
            acks: vec![],
            level: Level::default(),
            revision: 1,
            controller: Some(1),
            campaign_complete: false,
        })
        .unwrap();
        let decoded: ServerMessage = serde_json::from_str(&encoded).unwrap();
        let ServerMessage::Snapshot { world, .. } = decoded else {
            panic!("wrong variant")
        };
        assert_eq!(hash, world.state_hash());
    }

    #[test]
    fn chat_is_preset_only_and_server_worded() {
        let decoded = decode(r#"{"type":"chat","preset":2}"#).unwrap();
        assert!(matches!(decoded, ClientMessage::Chat { preset: 2 }));
        assert!(decode(r#"{"type":"chat","preset":0,"text":"hi"}"#).is_err());
        assert_eq!(CHAT_PRESETS.len(), 6);
        let encoded = serde_json::to_string(&ServerMessage::Chat {
            player: 2,
            preset: 2,
            text: CHAT_PRESETS[2].into(),
        })
        .unwrap();
        assert_eq!(
            encoded,
            r#"{"type":"chat","player":2,"preset":2,"text":"Jump!"}"#
        );
    }
}
