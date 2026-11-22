# Quick chat

Room-scoped preset messages, added in `30c47e3` (protocol, server, browser) and
`1c303e4` + `e250c72` (native wire and UI). This is the bounded social stream
the engine README called for; free text is deliberately absent.

## Presets

Defined once as `net_proto::CHAT_PRESETS` and mirrored by `Wire.chatPresets`.
The index is the wire `preset` value.

| Index | Text |
| --- | --- |
| 0 | Follow me |
| 1 | Wait here |
| 2 | Jump! |
| 3 | Ready? |
| 4 | Help! |
| 5 | Nice! |

## Server-owned wording

- Client → server is `{"type":"chat","preset":<0..5>}`
  (`ClientMessage::Chat { preset: u8 }`). The enum uses `deny_unknown_fields`,
  so a client that adds a `text` field fails to decode: free text cannot be
  sent even by a modified client.
- Server → clients is `ServerMessage::Chat { player, preset, text }`, where
  `text` is looked up from `CHAT_PRESETS[preset]`. The native decoder rejects
  any message whose `text` does not match the preset wording, whose preset is
  out of range, or whose player id is not positive.
- Because no user-supplied text exists anywhere on the wire, there is nothing
  to moderate, escape or filter: the moderation surface does not exist.

## Rate limit

- `CHAT_INTERVAL` in `crates/server` is **one message per player per 2 s**,
  tracked in the room slot's `last_chat`.
- `Room::chat` returns `None` for a message inside the interval, an unknown
  preset, or a disconnected slot: the message is **silently ignored** — no
  error reply, no kick, no protocol violation. Other players simply never see
  it.

## Transport: off the state path

- Each room owns a bounded `tokio::sync::broadcast` channel (capacity 16);
  every connection subscribes when it enters the room.
- `write_snapshots` drains it in the same `select!` loop that publishes
  snapshots and writes the chat frame straight to that socket. A lagged
  receiver skips the missed messages rather than erroring.
- Chat is transient: it is never carried in a `snapshot` or a `patch`, so it
  adds no bytes to the 20 Hz state path and no work to patch-base
  reconstruction.

## What each client renders

**Browser** (`engine/web/client.js`, `engine/web/index.html`)

- Six `[data-chat]` buttons above the movement controls send
  `{type:'chat', preset:n}` only while the socket is `OPEN`; clicks while
  disconnected send nothing.
- A received chat draws a bubble above that player's sprite for **3.5 s** and
  appends `P<n>: <text>` to `#chatlog`, which keeps the last **3 lines**
  joined by `·` (`aria-live="polite"`). Bubbles are cleared on (re)connect;
  the log is page memory only.

**Native** (`native/iOS`)

- `SessionModel.sendChat(preset:)` encodes via `ClientCommand.chat(preset:)`,
  which throws for a preset ≥ 6; incoming messages update `latestChat` and a
  3-entry `chatLog`.
- `CoopApp` shows a horizontally scrolling quick-chat row above the movement
  controls (`chat0`…`chat5` accessibility identifiers) and the same
  `·`-joined 3-line log.
- `GameScene.showChat` attaches a bubble label above the speaker's sprite for
  **3 s**, keyed by the entry timestamp so repeated snapshots do not re-show
  it. With Reduce Motion enabled the 0.2 s fade-in is skipped; a player with
  no sprite is skipped silently.

## Evidence

- Rust: `net-proto::chat_is_preset_only_and_server_worded` and
  `server::chat_presets_are_server_owned_and_rate_limited` (interval boundary,
  unknown preset).
- Browser: **40 Node tests** pass (`node --test web/*.test.mjs`), including
  preset-only send + server-worded render and send-while-disconnected.
- Native: **42 Swift tests** in the suite, including `ChatWireTests` (decode
  rejects mismatched wording and out-of-range presets; the command encodes
  `preset` only and rejects index ≥ 6).

## Not implemented

- No free text — preset index only, enforced by the protocol.
- No history persistence: logs are in-memory client state, never written to
  disk or stored server-side.
- No moderation, mute, block or report controls (and nothing to apply them to).
- No cross-room or direct messages; chat is room-scoped only.
- No profanity filtering — there is no user input to filter.
- No voice, typing indicators, read receipts or delivery guarantees beyond the
  room broadcast.
