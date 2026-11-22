//! Per-writer merge patches, never stored in the lossy latest-snapshot mailbox.
//! Arrays replace atomically; null is a value (not RFC7396 deletion).
use serde_json::{json, Map, Value};

fn difference(before: &Value, after: &Value) -> Value {
    match (before.as_object(), after.as_object()) {
        (Some(a), Some(b)) => Value::Object(
            b.iter()
                .filter_map(|(key, value)| {
                    if a.get(key) == Some(value) {
                        None
                    } else {
                        Some((
                            key.clone(),
                            a.get(key)
                                .map_or_else(|| value.clone(), |old| difference(old, value)),
                        ))
                    }
                })
                .collect::<Map<_, _>>(),
        ),
        _ => after.clone(),
    }
}

/// The patch grammar is add-free and delete-free by construction: both decoders
/// reject keys the base does not have and never drop keys the base does have, so
/// a key added to or removed from the snapshot cannot be expressed as a patch.
/// The encoder must therefore never emit either, and instead falls back to a
/// full snapshot whenever any object anywhere in the tree gains or loses a key.
fn same_key_shape(before: &Value, after: &Value) -> bool {
    match (before.as_object(), after.as_object()) {
        (Some(a), Some(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|next| same_key_shape(value, next)))
        }
        _ => true,
    }
}

pub fn encode(previous: Option<&Value>, text: &str) -> (String, Option<Value>) {
    let Ok(next) = serde_json::from_str::<Value>(text) else {
        return (text.into(), None);
    };
    let mut wire = text.to_owned();
    if let Some(old) =
        previous.filter(|old| old["revision"] == next["revision"] && old["level"] == next["level"])
    {
        if same_key_shape(old, &next) {
            let patch = json!({"type":"patch", "revision":next["revision"], "base_tick":old["world"]["tick"], "changes":difference(old, &next)}).to_string();
            if patch.len() < wire.len() {
                wire = patch;
            }
        }
    }
    (wire, Some(next))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measured_idle_payload_savings_cover_small_and_large_rooms() {
        use net_proto::ServerMessage;
        use sim_core::{campaign, World};
        for number in [1, 24] {
            let level = campaign::level(number).unwrap();
            let mut world = World::for_level(&level);
            for id in 1..=u32::from(level.required_players) {
                assert!(world.add_player_in(&level, id));
            }
            let mut previous = None;
            let (mut full_bytes, mut patch_bytes) = (0, 0);
            for tick in 0..1200 {
                world.step(&level, &[]);
                if tick % 3 != 0 {
                    continue;
                }
                let full = serde_json::to_string(&ServerMessage::Snapshot {
                    level: level.clone(),
                    world: world.clone(),
                    acks: vec![],
                    revision: 1,
                    controller: Some(1),
                    campaign_complete: false,
                })
                .unwrap();
                let (compact, next) = encode(previous.as_ref(), &full);
                full_bytes += full.len();
                patch_bytes += compact.len();
                previous = next;
            }
            assert!(patch_bytes < full_bytes / 2);
            println!("stage {number}: 20s idle payload full={full_bytes} compact={patch_bytes} bytes ({:.1}% lower)", 100.0 * (1.0 - patch_bytes as f64 / full_bytes as f64));
        }
    }
    #[test]
    fn patches_use_last_written_base_and_full_frames_for_revision_changes() {
        let a = json!({"type":"snapshot","revision":1,"level":{"solids":vec![42;100]},"world":{"tick":1,"checkpoint":null},"acks":[]}).to_string();
        let (_, base) = encode(None, &a);
        let b = a.replace("\"tick\":1", "\"tick\":7"); // Skipped mailbox ticks are fine.
        let (patch, _) = encode(base.as_ref(), &b);
        let patch: Value = serde_json::from_str(&patch).unwrap();
        assert_eq!(patch["type"], "patch");
        assert_eq!(patch["base_tick"], 1);
        assert_eq!(patch["changes"]["world"]["tick"], 7);
        assert!(patch["changes"].get("level").is_none());
        let c = b.replace("\"revision\":1", "\"revision\":2");
        assert_eq!(encode(base.as_ref(), &c).0, c);
    }
    #[test]
    fn added_key_forces_full_snapshot() {
        let a = json!({"type":"snapshot","revision":1,"level":{"solids":vec![42;100]},"world":{"tick":1,"checkpoint":null},"acks":[]}).to_string();
        let (_, base) = encode(None, &a);
        let b = json!({"type":"snapshot","revision":1,"level":{"solids":vec![42;100]},"world":{"tick":7,"checkpoint":null,"spawn":3},"acks":[]}).to_string();
        let (wire, _) = encode(base.as_ref(), &b);
        assert_eq!(wire, b, "a key the base lacks cannot be patched, so the full snapshot is the wire form");
        assert_eq!(serde_json::from_str::<Value>(&wire).unwrap()["type"], "snapshot");
    }
    #[test]
    fn removed_key_forces_full_snapshot() {
        let a = json!({"type":"snapshot","revision":1,"level":{"solids":vec![42;100]},"world":{"tick":1,"checkpoint":null},"acks":[]}).to_string();
        let (_, base) = encode(None, &a);
        let b = json!({"type":"snapshot","revision":1,"level":{"solids":vec![42;100]},"world":{"tick":7},"acks":[]}).to_string();
        let (wire, _) = encode(base.as_ref(), &b);
        assert_eq!(wire, b, "a key only the base has cannot be patched, so the full snapshot is the wire form");
        assert_eq!(serde_json::from_str::<Value>(&wire).unwrap()["type"], "snapshot");
    }
    #[test]
    fn unchanged_key_set_still_patches() {
        let a = json!({"type":"snapshot","revision":1,"level":{"solids":vec![42;100]},"world":{"tick":1,"checkpoint":null},"acks":[]}).to_string();
        let (_, base) = encode(None, &a);
        let b = a.replace("\"tick\":1", "\"tick\":7");
        let (wire, _) = encode(base.as_ref(), &b);
        let wire: Value = serde_json::from_str(&wire).unwrap();
        assert_eq!(wire["type"], "patch");
        assert_eq!(wire["changes"]["world"]["tick"], 7);
        assert!(wire["changes"].get("level").is_none());
    }
}
