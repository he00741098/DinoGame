//! Bounded, development-only authoritative room host. No legacy Dino dependencies.
#[cfg(test)]
mod campaign_tests;
mod snapshot_patch;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use net_proto::{ClientMessage, InputAck, ServerMessage, PROTOCOL_VERSION, RULESET};
use sim_core::{campaign, Input, Level, PlayerId, World, MAX_PLAYERS, TICK_HZ};
use tokio::{
    sync::{broadcast, watch, Mutex, Semaphore},
    time::{timeout, Instant, MissedTickBehavior},
};

const MAX_ROOMS: usize = 64;
// Bounded upgrade headroom lets a full host accept replacements before old sockets exit.
const MAX_CONNECTIONS: usize = MAX_ROOMS * MAX_PLAYERS + 64;
const SNAPSHOT_HZ: u32 = 20;
const AUTO_ADVANCE_DELAY: Duration = Duration::from_millis(900);
const INPUTS_PER_SECOND: usize = 120;
// Chat is best-effort, so it gets its own tight budget: spam can never spend
// the gameplay budget, and blowing it drops frames instead of the player.
const CHAT_FRAMES_PER_SECOND: usize = 8;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const INPUT_LEASE: Duration = Duration::from_millis(250);
const RESUME_GRACE: Duration = Duration::from_secs(15);
const CHAT_INTERVAL: Duration = Duration::from_secs(2);
const PROGRESS_TTL: Duration = Duration::from_secs(3600);
const MAX_SAVED_ROOMS: usize = 128;
// Never reused, even if a room is deleted/recreated while an old socket unwinds.
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

fn next_generation() -> Result<u64, &'static str> {
    NEXT_GENERATION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| "generation_exhausted")
}

struct Slot {
    tx: watch::Sender<String>,
    stop: watch::Sender<bool>,
    token: String,
    generation: u64,
    connected: bool,
    disconnected_at: Option<Instant>,
    latest_seq: u32,
    ack: Option<u32>,
    jump_id: u32,
    jump_pending: bool,
    fresh_at: Option<Instant>,
    pending: Option<(u32, Input)>,
    held: Input,
    needs_jump_baseline: bool,
    last_chat: Option<Instant>,
}

impl Slot {
    fn neutralize(&mut self) {
        self.pending = None;
        self.held = Input::default();
        self.jump_pending = false;
        self.fresh_at = None;
    }
}

struct Joined {
    player: PlayerId,
    generation: u64,
    token: String,
    resumed: bool,
    snapshot: ServerMessage,
    chat: broadcast::Receiver<ServerMessage>,
}

struct Room {
    world: World,
    level: Level,
    revision: u32,
    slots: BTreeMap<PlayerId, Slot>,
    next_id: PlayerId,
    completed_at: Option<Instant>,
    final_signature: Option<String>,
    chat: broadcast::Sender<ServerMessage>,
}

#[derive(Clone)]
struct SavedProgress {
    number: u8,
    completed: bool,
    expires: Instant,
}

fn save_progress(
    cache: &mut BTreeMap<String, SavedProgress>,
    name: &str,
    room: &Room,
    now: Instant,
) {
    cache.retain(|_, value| value.expires > now);
    cache.insert(
        name.to_owned(),
        SavedProgress {
            number: room.level.number,
            completed: room.world.completed,
            expires: now + PROGRESS_TTL,
        },
    );
    while cache.len() > MAX_SAVED_ROOMS {
        let oldest = cache
            .iter()
            .min_by_key(|(_, value)| value.expires)
            .map(|(key, _)| key.clone());
        if let Some(oldest) = oldest {
            cache.remove(&oldest);
        }
    }
}

impl Room {
    #[cfg(test)]
    fn new() -> Self {
        Self::from_progress(None)
    }

    fn from_progress(saved: Option<SavedProgress>) -> Self {
        let level = saved
            .as_ref()
            .and_then(|saved| campaign::level(saved.number))
            .unwrap_or_else(|| campaign::level(1).expect("bundled campaign starts at level 1"));
        let mut world = World::for_level(&level);
        if saved.is_some_and(|saved| saved.completed) {
            world.key_collected = true;
            world.completed = true;
        }
        Self {
            world,
            level,
            revision: 1,
            slots: BTreeMap::new(),
            next_id: 1,
            completed_at: None,
            final_signature: None,
            chat: broadcast::channel(16).0,
        }
    }

    fn join(
        &mut self,
        tx: watch::Sender<String>,
        stop: watch::Sender<bool>,
        token: Option<&str>,
        now: Instant,
    ) -> Result<Joined, &'static str> {
        self.expire(now);
        if let Some(token) = token {
            let (&id, slot) = self
                .slots
                .iter_mut()
                .find(|(_, s)| s.token == token)
                .ok_or("invalid_resume_token")?;
            let generation = next_generation()?;
            slot.stop.send_replace(true);
            slot.tx = tx;
            slot.stop = stop;
            slot.generation = generation;
            slot.connected = true;
            slot.disconnected_at = None;
            slot.neutralize();
            slot.latest_seq = 0;
            slot.ack = None;
            slot.jump_id = 0;
            slot.needs_jump_baseline = false;
            self.stop_motion(id);
            return Ok(Joined {
                player: id,
                generation,
                token: token.to_owned(),
                resumed: true,
                snapshot: self.snapshot(),
                chat: self.chat.subscribe(),
            });
        }
        if self.slots.len() >= MAX_PLAYERS {
            return Err("room_full");
        }
        let id = self.next_id;
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| "random_unavailable")?;
        let token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        let generation = next_generation()?;
        self.next_id = self.next_id.checked_add(1).ok_or("id_exhausted")?;
        if !self.world.add_player_in(&self.level, id) {
            return Err("room_full");
        }
        self.slots.insert(
            id,
            Slot {
                tx,
                stop,
                token: token.clone(),
                generation,
                connected: true,
                disconnected_at: None,
                latest_seq: 0,
                ack: None,
                jump_id: 0,
                jump_pending: false,
                fresh_at: None,
                pending: None,
                held: Input::default(),
                needs_jump_baseline: false,
                last_chat: None,
            },
        );
        Ok(Joined {
            player: id,
            generation,
            token,
            resumed: false,
            snapshot: self.snapshot(),
            chat: self.chat.subscribe(),
        })
    }

    fn input(
        &mut self,
        player: PlayerId,
        generation: u64,
        seq: u32,
        input: Input,
        jump_id: u32,
        now: Instant,
    ) -> bool {
        if let Some(slot) = self.slots.get_mut(&player) {
            if slot.connected
                && generation == slot.generation
                && seq > slot.latest_seq
                && input.valid()
            {
                if slot
                    .fresh_at
                    .is_some_and(|at| now.duration_since(at) >= INPUT_LEASE)
                {
                    slot.neutralize();
                }
                slot.latest_seq = seq;
                slot.fresh_at = Some(now);
                // One pending edge, not a count: skipped IDs never synthesize future work.
                let baseline = slot.needs_jump_baseline;
                if baseline {
                    slot.jump_id = jump_id;
                    slot.needs_jump_baseline = false;
                } else if jump_id > slot.jump_id {
                    slot.jump_pending = true;
                    slot.jump_id = jump_id;
                }
                slot.pending = Some((seq, if baseline { Input::default() } else { input }));
                return true;
            }
        }
        false
    }

    fn command(
        &mut self,
        player: PlayerId,
        generation: u64,
        command: ClientMessage,
        now: Instant,
    ) -> bool {
        match command {
            ClientMessage::Input {
                seq,
                input,
                jump_id,
                revision,
            } if revision == self.revision => {
                self.input(player, generation, seq, input, jump_id, now)
            }
            ClientMessage::Restart { seq, revision } => {
                self.control(player, generation, seq, revision, false)
            }
            ClientMessage::Next { seq, revision } => {
                self.control(player, generation, seq, revision, true)
            }
            _ => false,
        }
    }

    /// Server-owned wording; the client only sends a preset index. Silently
    /// ignores unknown presets, disconnected slots and messages inside the
    /// per-player interval.
    fn chat(&mut self, player: PlayerId, preset: u8, now: Instant) -> Option<ServerMessage> {
        let text = net_proto::CHAT_PRESETS.get(preset as usize)?;
        let slot = self.slots.get_mut(&player)?;
        if !slot.connected
            || slot
                .last_chat
                .is_some_and(|last| now.duration_since(last) < CHAT_INTERVAL)
        {
            return None;
        }
        slot.last_chat = Some(now);
        Some(ServerMessage::Chat {
            player,
            preset,
            text: (*text).into(),
        })
    }

    fn controller(&self) -> Option<PlayerId> {        self.slots
            .iter()
            .find(|(_, slot)| slot.connected)
            .map(|(&id, _)| id)
    }

    /// One bounded transition, authorized by identity/generation and current round.
    /// Duplicate, stale, non-controller and prematurely queued requests are no-ops.
    fn control(
        &mut self,
        player: PlayerId,
        generation: u64,
        seq: u32,
        revision: u32,
        next: bool,
    ) -> bool {
        if revision != self.revision
            || self.controller() != Some(player)
            || !self.slots.get(&player).is_some_and(|slot| {
                slot.connected && slot.generation == generation && seq > slot.latest_seq
            })
        {
            return false;
        }
        if next && (!self.world.completed || self.level.number >= campaign::LEVEL_COUNT) {
            return false;
        }
        let number = if next {
            self.level.number + 1
        } else {
            self.level.number
        };
        if !self.transition(number) {
            return false;
        }
        if let Some(slot) = self.slots.get_mut(&player) {
            slot.latest_seq = seq;
            slot.ack = Some(seq);
        }
        true
    }

    /// Shared authoritative transition for manual restart/next and timed advance.
    fn transition(&mut self, number: u8) -> bool {
        let Some(new_revision) = self.revision.checked_add(1) else {
            return false;
        };
        let Some(level) = campaign::level(number) else {
            return false;
        };
        let mut world = World::for_level(&level);
        world.tick = self.world.tick;
        world.level_start_tick = world.tick;
        for &id in self.slots.keys() {
            if !world.add_player_in(&level, id) {
                return false;
            }
        }
        self.world = world;
        self.level = level;
        self.revision = new_revision;
        self.completed_at = None;
        self.final_signature = None;
        for slot in self.slots.values_mut() {
            slot.neutralize();
            slot.needs_jump_baseline = true;
            slot.ack = None;
        }
        true
    }

    fn stop_motion(&mut self, id: PlayerId) {
        if let Some(p) = self.world.players.iter_mut().find(|p| p.id == id) {
            p.vx = 0;
            p.jump_held = false;
        }
    }

    fn leave(&mut self, player: PlayerId, generation: u64, permanent: bool, now: Instant) {
        let Some(slot) = self.slots.get_mut(&player) else {
            return;
        };
        if slot.generation != generation || !slot.connected {
            return;
        }
        slot.neutralize();
        slot.connected = false;
        slot.disconnected_at = Some(now);
        self.stop_motion(player);
        if permanent {
            self.slots.remove(&player);
            self.world.remove_player(player);
        }
    }

    fn expire(&mut self, now: Instant) {
        let expired: Vec<_> = self
            .slots
            .iter()
            .filter(|(_, s)| {
                s.disconnected_at
                    .is_some_and(|at| now.duration_since(at) >= RESUME_GRACE)
            })
            .map(|(&id, _)| id)
            .collect();
        for id in expired {
            self.slots.remove(&id);
            self.world.remove_player(id);
        }
    }

    fn tick(&mut self, level: &Level, now: Instant) {
        self.expire(now);
        let ready = self.slots.values().all(|s| s.connected);
        let inputs: Vec<_> = self
            .slots
            .iter_mut()
            .map(|(&id, slot)| {
                if !slot.connected
                    || slot
                        .fresh_at
                        .is_none_or(|at| now.duration_since(at) >= INPUT_LEASE)
                {
                    slot.neutralize();
                }
                if let Some((seq, input)) = slot.pending.take() {
                    slot.held = input;
                    slot.ack = Some(seq);
                }
                let edge = std::mem::take(&mut slot.jump_pending);
                (id, slot.held, edge)
            })
            .collect();
        self.world.step_with_jump_edges(level, &inputs, ready);
        // Completed worlds do not run physics, but leases must still stop old movement.
        for (&id, slot) in &self.slots {
            if slot.fresh_at.is_none() {
                if let Some(p) = self.world.players.iter_mut().find(|p| p.id == id) {
                    p.vx = 0;
                    p.jump_held = false;
                }
            }
        }
        if self.world.completed
            && ready
            && self.slots.len() >= 2
            && self.level.number < campaign::LEVEL_COUNT
        {
            let completed_at = *self.completed_at.get_or_insert(now);
            if now.duration_since(completed_at) >= AUTO_ADVANCE_DELAY {
                self.transition(self.level.number + 1);
            }
        } else {
            self.completed_at = None;
        }
    }

    fn snapshot(&self) -> ServerMessage {
        ServerMessage::Snapshot {
            world: self.world.clone(),
            level: self.level.clone(),
            revision: self.revision,
            controller: self.controller(),
            campaign_complete: self.world.completed && self.level.number == campaign::LEVEL_COUNT,
            acks: self
                .slots
                .iter()
                .filter_map(|(&player, s)| s.ack.map(|seq| InputAck { player, seq }))
                .collect(),
        }
    }

    fn publication(&mut self) -> Option<String> {
        let snapshot = self.snapshot();
        if self.world.completed && self.level.number == campaign::LEVEL_COUNT {
            let mut value = serde_json::to_value(&snapshot).ok()?;
            value["world"]["tick"] = 0.into();
            let signature = value.to_string();
            if self.final_signature.as_ref() == Some(&signature) {
                return None;
            }
            self.final_signature = Some(signature);
        } else {
            self.final_signature = None;
        }
        serde_json::to_string(&snapshot).ok()
    }
}

#[derive(Clone)]
pub struct Host {
    rooms: Arc<Mutex<BTreeMap<String, Room>>>,
    connections: Arc<Semaphore>,
    progress: Arc<Mutex<BTreeMap<String, SavedProgress>>>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            rooms: Arc::new(Mutex::new(BTreeMap::new())),
            connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
            progress: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

impl Host {
    async fn join(
        &self,
        room: &str,
        tx: watch::Sender<String>,
        stop: watch::Sender<bool>,
        token: Option<&str>,
    ) -> Result<Joined, &'static str> {
        let mut rooms = self.rooms.lock().await;
        if token.is_some() && !rooms.contains_key(room) {
            return Err("invalid_resume_token");
        }
        if !rooms.contains_key(room) && rooms.len() >= MAX_ROOMS {
            return Err("server_full");
        }
        if !rooms.contains_key(room) {
            let now = Instant::now();
            let mut progress = self.progress.lock().await;
            progress.retain(|_, saved| saved.expires > now);
            let saved = progress.remove(room);
            rooms.insert(room.to_owned(), Room::from_progress(saved));
        }
        rooms
            .get_mut(room)
            .expect("room just checked or inserted")
            .join(tx, stop, token, Instant::now())
    }

    async fn leave(&self, room: &str, player: PlayerId, generation: u64, permanent: bool) {
        let mut rooms = self.rooms.lock().await;
        if let Some(state) = rooms.get_mut(room) {
            state.leave(player, generation, permanent, Instant::now());
            if state.slots.is_empty() {
                save_progress(
                    &mut *self.progress.lock().await,
                    room,
                    state,
                    Instant::now(),
                );
                rooms.remove(room);
            }
        }
    }

    pub async fn run_ticks(&self) {
        let mut timer = tokio::time::interval(Duration::from_secs_f64(1.0 / f64::from(TICK_HZ)));
        // No unbounded catch-up bursts after a stalled host. Simulation slows under overload.
        timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            let mut rooms = self.rooms.lock().await;
            for room in rooms.values_mut() {
                let level = room.level.clone();
                room.tick(&level, Instant::now());
                if room.world.tick % u64::from(TICK_HZ / SNAPSHOT_HZ) == 0 {
                    if let Some(snapshot) = room.publication() {
                        for slot in room.slots.values() {
                            if slot.connected {
                                // watch replaces the old snapshot, never discards the newest.
                                slot.tx.send_replace(snapshot.clone());
                            }
                        }
                    }
                }
            }
            let mut progress = self.progress.lock().await;
            for (name, room) in rooms.iter().filter(|(_, room)| room.slots.is_empty()) {
                save_progress(&mut progress, name, room, Instant::now());
            }
            progress.retain(|_, saved| saved.expires > Instant::now());
            rooms.retain(|_, room| !room.slots.is_empty());
        }
    }
}

pub fn app(host: Host) -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../../../web/index.html")) }),
        )
        .route(
            "/client.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../../web/client.js"),
                )
            }),
        )
        .route(
            "/network.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../../web/network.js"),
                )
            }),
        )
        .route("/solutions/{number}", get(solution))
        .route(
            "/motion.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../../web/motion.js"),
                )
            }),
        )
        .route("/health", get(|| async { "ok" }))
        .route(
            "/campaign",
            get(|| async {
                axum::Json(
                    (1..=campaign::LEVEL_COUNT)
                        .filter_map(campaign::level)
                        .collect::<Vec<_>>(),
                )
            }),
        )
        .route("/ws", get(upgrade))
        .with_state(host)
}

async fn upgrade(State(host): State<Host>, ws: WebSocketUpgrade) -> Response {
    let Ok(permit) = host.connections.clone().try_acquire_owned() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "server full").into_response();
    };
    ws.max_message_size(net_proto::MAX_MESSAGE_BYTES)
        .max_frame_size(net_proto::MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            session(socket, host).await;
        })
}

async fn solution(Path(number): Path<u8>) -> Response {
    const RECORDINGS: [&str; 32] = [
        include_str!("../../../web/solutions/1.json"),
        include_str!("../../../web/solutions/2.json"),
        include_str!("../../../web/solutions/3.json"),
        include_str!("../../../web/solutions/4.json"),
        include_str!("../../../web/solutions/5.json"),
        include_str!("../../../web/solutions/6.json"),
        include_str!("../../../web/solutions/7.json"),
        include_str!("../../../web/solutions/8.json"),
        include_str!("../../../web/solutions/9.json"),
        include_str!("../../../web/solutions/10.json"),
        include_str!("../../../web/solutions/11.json"),
        include_str!("../../../web/solutions/12.json"),
        include_str!("../../../web/solutions/13.json"),
        include_str!("../../../web/solutions/14.json"),
        include_str!("../../../web/solutions/15.json"),
        include_str!("../../../web/solutions/16.json"),
        include_str!("../../../web/solutions/17.json"),
        include_str!("../../../web/solutions/18.json"),
        include_str!("../../../web/solutions/19.json"),
        include_str!("../../../web/solutions/20.json"),
        include_str!("../../../web/solutions/21.json"),
        include_str!("../../../web/solutions/22.json"),
        include_str!("../../../web/solutions/23.json"),
        include_str!("../../../web/solutions/24.json"),
        include_str!("../../../web/solutions/25.json"),
        include_str!("../../../web/solutions/26.json"),
        include_str!("../../../web/solutions/27.json"),
        include_str!("../../../web/solutions/28.json"),
        include_str!("../../../web/solutions/29.json"),
        include_str!("../../../web/solutions/30.json"),
        include_str!("../../../web/solutions/31.json"),
        include_str!("../../../web/solutions/32.json"),
    ];
    match number
        .checked_sub(1)
        .and_then(|n| RECORDINGS.get(n as usize))
    {
        Some(recording) => ([("content-type", "application/json")], *recording).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn send(socket: &mut WebSocket, message: &ServerMessage) -> bool {
    let Ok(text) = serde_json::to_string(message) else {
        return false;
    };
    matches!(
        timeout(IO_TIMEOUT, socket.send(Message::Text(text.into()))).await,
        Ok(Ok(()))
    )
}

async fn error(socket: &mut WebSocket, code: &str) {
    let _ = send(
        socket,
        &ServerMessage::Error {
            code: code.into(),
            message: code.replace('_', " "),
        },
    )
    .await;
}

async fn session(mut socket: WebSocket, host: Host) {
    let hello = match timeout(IO_TIMEOUT, socket.recv()).await {
        Ok(Some(Ok(Message::Text(text)))) => net_proto::decode(&text),
        _ => {
            error(&mut socket, "hello_required").await;
            return;
        }
    };
    let hello = match hello {
        Ok(message) => message,
        Err(_) => {
            error(&mut socket, "invalid_message").await;
            return;
        }
    };
    let room = match net_proto::validate_hello(&hello) {
        Ok(room) => room.to_owned(),
        Err(code) => {
            error(&mut socket, code).await;
            return;
        }
    };
    let token = match &hello {
        ClientMessage::Hello { resume_token, .. } => resume_token.as_deref(),
        _ => None,
    };
    let patches = matches!(&hello, ClientMessage::Hello { patches: true, .. });
    let (tx, rx) = watch::channel(String::new());
    let (stop, stopped) = watch::channel(false);
    let joined = match host.join(&room, tx, stop, token).await {
        Ok(joined) => joined,
        Err(code) => {
            error(&mut socket, code).await;
            return;
        }
    };
    let ServerMessage::Snapshot {
        level,
        revision,
        controller,
        ..
    } = &joined.snapshot
    else {
        unreachable!("joining always produces a full snapshot");
    };
    let welcome = ServerMessage::Welcome {
        protocol: PROTOCOL_VERSION,
        ruleset: RULESET.into(),
        player: joined.player,
        tick_hz: TICK_HZ,
        snapshot_hz: SNAPSHOT_HZ,
        level: level.clone(),
        revision: *revision,
        controller: *controller,
        resume_token: joined.token,
        resumed: joined.resumed,
    };
    let (mut sink, mut stream) = socket.split();
    let initial = [
        serde_json::to_string(&welcome).unwrap(),
        serde_json::to_string(&joined.snapshot).unwrap(),
    ];
    Connection {
        host: &host,
        room: &room,
        player: joined.player,
        generation: joined.generation,
    }
    .run(
        &mut sink,
        &mut stream,
        Writer {
            rx,
            stopped,
            chat: joined.chat,
            initial,
            patches,
        },
    )
    .await;
}

struct Connection<'a> {
    host: &'a Host,
    room: &'a str,
    player: PlayerId,
    generation: u64,
}

// The writer half of a connection: everything `write_snapshots` and the stop
// branch need, grouped so `run` stays within clippy's argument limit.
struct Writer {
    rx: watch::Receiver<String>,
    stopped: watch::Receiver<bool>,
    chat: broadcast::Receiver<ServerMessage>,
    initial: [String; 2],
    patches: bool,
}

impl Connection<'_> {
    // Shared by real sockets and bounded blocked-I/O session regression tests.
    async fn run<S, R>(&self, sink: &mut S, stream: &mut R, writer: Writer)
    where
        S: futures_util::Sink<Message> + Unpin,
        R: futures_util::Stream<Item = Result<Message, axum::Error>> + Unpin,
    {
        let Writer {
            mut stopped,
            rx,
            chat,
            initial,
            patches,
        } = writer;
        let outcome = tokio::select! {
            biased;
            _ = stopped.changed() => Outcome::disconnected(),
            outcome = read_inputs(stream, self.host, self.room, self.player, self.generation) => outcome,
            _ = write_snapshots(sink, rx, chat, initial, patches) => Outcome::disconnected(),
        };
        // Neutralize before any possibly blocked error/close write. Stale generation is a no-op.
        self.host
            .leave(self.room, self.player, self.generation, outcome.permanent)
            .await;
        if let Some(code) = outcome.error {
            let text = serde_json::to_string(&ServerMessage::Error {
                code: code.into(),
                message: code.replace('_', " "),
            })
            .unwrap();
            let _ = timeout(IO_TIMEOUT, sink.send(Message::Text(text.into()))).await;
        }
        let _ = timeout(IO_TIMEOUT, sink.close()).await;
    }
}

struct Outcome {
    permanent: bool,
    error: Option<&'static str>,
}

impl Outcome {
    fn disconnected() -> Self {
        Self {
            permanent: false,
            error: None,
        }
    }
    fn invalid(code: &'static str) -> Self {
        Self {
            permanent: true,
            error: Some(code),
        }
    }
}

fn read_failure(error: &axum::Error) -> Outcome {
    use std::error::Error;
    use tungstenite::{error::ProtocolError, Error as WsError};
    let mut source = error.source();
    while let Some(cause) = source {
        if let Some(ws) = cause.downcast_ref::<WsError>() {
            // Tungstenite reports a bare transport EOF as a protocol error, not Io.
            return match ws {
                WsError::Io(_)
                | WsError::Tls(_)
                | WsError::ConnectionClosed
                | WsError::AlreadyClosed
                | WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake) => {
                    Outcome::disconnected()
                }
                _ => Outcome::invalid("invalid_websocket"),
            };
        }
        source = cause.source();
    }
    // Unknown read failures fail closed rather than retaining a potentially invalid peer.
    Outcome::invalid("invalid_websocket")
}

async fn write_snapshots<S: futures_util::Sink<Message> + Unpin>(
    sink: &mut S,
    mut rx: watch::Receiver<String>,
    mut chat: broadcast::Receiver<ServerMessage>,
    initial: [String; 2],
    patches: bool,
) {
    let mut previous = None;
    for text in initial {
        if patches && text.contains("\"type\":\"snapshot\"") {
            previous = serde_json::from_str(&text).ok();
        }
        if !matches!(
            timeout(IO_TIMEOUT, sink.send(Message::Text(text.into()))).await,
            Ok(Ok(()))
        ) {
            return;
        }
    }
    loop {
        tokio::select! {
            changed = rx.changed() => {
                if changed.is_err() {
                    return;
                }
                let text = rx.borrow_and_update().clone();
                let (text, next) = if patches {
                    snapshot_patch::encode(previous.as_ref(), &text)
                } else {
                    (text, None)
                };
                if !matches!(
                    timeout(IO_TIMEOUT, sink.send(Message::Text(text.into()))).await,
                    Ok(Ok(()))
                ) {
                    return;
                }
                previous = next;
            }
            // Chat is transient and never part of a snapshot or patch.
            event = chat.recv() => match event {
                Ok(message) => {
                    let text = serde_json::to_string(&message).unwrap();
                    if !matches!(
                        timeout(IO_TIMEOUT, sink.send(Message::Text(text.into()))).await,
                        Ok(Ok(()))
                    ) {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            },
        }
    }
}

async fn read_inputs<S: futures_util::Stream<Item = Result<Message, axum::Error>> + Unpin>(
    stream: &mut S,
    host: &Host,
    room: &str,
    player: PlayerId,
    generation: u64,
) -> Outcome {
    let mut rate_start = Instant::now();
    let mut received = 0;
    let mut chat_received = 0;
    let mut last_input = Instant::now();
    let mut watchdog = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            _ = watchdog.tick() => {
                if last_input.elapsed() > Duration::from_secs(30) {
                    return Outcome { permanent: false, error: Some("idle_timeout") };
                }
            }
            incoming = stream.next() => {
                if rate_start.elapsed() >= Duration::from_secs(1) {
                    rate_start = Instant::now();
                    received = 0;
                    chat_received = 0;
                }
                let decoded = match &incoming {
                    Some(Ok(Message::Text(text))) => net_proto::decode(text).ok(),
                    _ => None,
                };
                if matches!(decoded, Some(ClientMessage::Chat { .. })) {
                    chat_received += 1;
                    if chat_received > CHAT_FRAMES_PER_SECOND {
                        continue;
                    }
                } else {
                    received += 1;
                    if received > INPUTS_PER_SECOND {
                        return Outcome::invalid("rate_limited");
                    }
                }
                match incoming {
                    Some(Ok(Message::Text(_))) => match decoded {
                        Some(command @ (ClientMessage::Input { .. } | ClientMessage::Restart { .. } | ClientMessage::Next { .. })) => {
                            if let Some(state) = host.rooms.lock().await.get_mut(room) {
                                if state.command(player, generation, command, Instant::now()) {
                                    last_input = Instant::now();
                                }
                            }
                        }
                        Some(ClientMessage::Chat { preset }) => {
                            if let Some(state) = host.rooms.lock().await.get_mut(room) {
                                if let Some(message) = state.chat(player, preset, Instant::now()) {
                                    let _ = state.chat.send(message);
                                }
                            }
                        }
                        _ => return Outcome::invalid("invalid_message"),
                    },
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {},
                    Some(Ok(Message::Close(frame))) => return Outcome { permanent: frame.is_some_and(|f| f.code == 1000), error: None },
                    Some(Err(error)) => return read_failure(&error),
                    None => return Outcome::disconnected(),
                    _ => return Outcome::invalid("text_required"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn join(room: &mut Room, token: Option<&str>, now: Instant) -> Joined {
        room.join(
            watch::channel(String::new()).0,
            watch::channel(false).0,
            token,
            now,
        )
        .unwrap()
    }

    #[test]
    fn chat_presets_are_server_owned_and_rate_limited() {
        let now = Instant::now();
        let mut room = Room::new();
        let a = join(&mut room, None, now);
        assert!(matches!(
            room.chat(a.player, 0, now),
            Some(ServerMessage::Chat { text, .. }) if text == "Follow me"
        ));
        assert!(room
            .chat(a.player, 1, now + Duration::from_millis(500))
            .is_none());
        assert!(room.chat(a.player, 1, now + CHAT_INTERVAL).is_some());
        assert!(room
            .chat(a.player, 200, now + Duration::from_secs(10))
            .is_none());
    }

    #[tokio::test]
    async fn chat_broadcast_reaches_subscriber_without_snapshot_dependency() {
        let now = Instant::now();
        let mut room = Room::new();
        // Keep the snapshot receiver so the test can prove chat never touches it.
        let (tx, snapshots) = watch::channel(String::new());
        let joined = room.join(tx, watch::channel(false).0, None, now).unwrap();
        let mut chat = joined.chat;
        let writer_chat = room.chat.subscribe();
        room.chat
            .send(ServerMessage::Chat {
                player: joined.player,
                preset: 0,
                text: "Follow me".into(),
            })
            .unwrap();

        match chat.recv().await {
            Ok(ServerMessage::Chat {
                player,
                preset,
                text,
            }) => {
                assert_eq!(player, joined.player);
                assert_eq!(preset, 0);
                assert_eq!(text, "Follow me");
            }
            other => panic!("expected the sent chat message, got {other:?}"),
        }
        // Exactly one message: chat is not bundled with a snapshot or anything else.
        assert!(matches!(
            chat.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        // No tick ran, so there is no snapshot publication to depend on.
        assert_eq!(*snapshots.borrow(), "");

        // The socket writer forwards chat from its own subscription without snapshots.
        let (out, mut captured) = tokio::sync::mpsc::unbounded_channel();
        let sink = futures_util::sink::unfold((), move |(), message: Message| {
            let out = out.clone();
            async move {
                let _ = out.send(message);
                Ok::<(), std::io::Error>(())
            }
        });
        tokio::pin!(sink);
        let mut forwarded = false;
        tokio::select! {
            () = write_snapshots(
                &mut sink,
                snapshots.clone(),
                writer_chat,
                ["welcome".into(), "snapshot".into()],
                false,
            ) => panic!("writer finished without a closed snapshot sender"),
            _ = async {
                while !forwarded {
                    let Some(message) = captured.recv().await else { break; };
                    if let Message::Text(text) = message {
                        if let Ok(ServerMessage::Chat {
                            player,
                            preset,
                            text: wording,
                        }) = serde_json::from_str(&text)
                        {
                            assert_eq!(player, joined.player);
                            assert_eq!(preset, 0);
                            assert_eq!(wording, "Follow me");
                            forwarded = true;
                        }
                    }
                }
            } => {}
        }
        assert!(
            forwarded,
            "chat must reach the socket without a snapshot publication"
        );
        assert!(
            !snapshots.has_changed().expect("snapshot sender stays alive"),
            "chat delivery must not publish a snapshot"
        );
    }

    #[test]
    fn chat_stays_rate_limited_per_player_not_per_room() {
        let now = Instant::now();
        let mut room = Room::new();
        let a = join(&mut room, None, now);
        let b = join(&mut room, None, now);
        let soon = now + Duration::from_millis(500);

        let first = room.chat(a.player, 0, now);
        assert!(matches!(
            first,
            Some(ServerMessage::Chat { player, preset: 0, text })
                if player == a.player && text == "Follow me"
        ));
        assert_eq!(room.slots[&a.player].last_chat, Some(now));

        // The same player speaking again inside CHAT_INTERVAL is rejected.
        assert!(room.chat(a.player, 1, soon).is_none());
        assert_eq!(
            room.slots[&a.player].last_chat,
            Some(now),
            "a rejected chat must not advance last_chat"
        );

        // The interval is per player, so player 2 is unaffected in the same room.
        let second = room.chat(b.player, 1, soon);
        assert!(matches!(
            second,
            Some(ServerMessage::Chat { player, preset: 1, text })
                if player == b.player && text == "Wait here"
        ));
        assert_eq!(room.slots[&b.player].last_chat, Some(soon));
        assert_eq!(room.slots[&a.player].last_chat, Some(now));
    }

    #[test]
    fn input_is_acknowledged_only_when_applied_and_stale_is_ignored() {
        let mut room = Room::new();
        let now = Instant::now();
        let joined = join(&mut room, None, now);
        let id = joined.player;
        let generation = joined.generation;
        let x = room.world.players[0].x;
        room.input(
            id,
            generation,
            2,
            Input {
                axis: 1,
                jump: false,
            },
            0,
            now,
        );
        room.input(
            id,
            generation,
            1,
            Input {
                axis: -1,
                jump: false,
            },
            1,
            now + INPUT_LEASE,
        );
        assert_eq!(room.slots[&id].ack, None);
        assert_eq!(room.slots[&id].fresh_at, Some(now));
        assert_eq!(room.world.tick, 0);
        room.tick(&Level::default(), now);
        assert_eq!(room.slots[&id].ack, Some(2));
        assert_eq!(room.world.players[0].x, x + 96);
        room.tick(&Level::default(), now + INPUT_LEASE);
        assert_eq!(room.world.players[0].vx, 0);
        assert_eq!(room.world.players[0].x, x + 96);
    }

    #[test]
    fn short_taps_are_preserved_but_pending_edges_are_bounded_and_never_burst() {
        let mut room = Room::new();
        let now = Instant::now();
        let joined = join(&mut room, None, now);
        let id = joined.player;
        let generation = joined.generation;
        room.input(
            id,
            generation,
            1,
            Input {
                axis: 0,
                jump: true,
            },
            1,
            now,
        );
        room.input(id, generation, 2, Input::default(), 1, now);
        room.tick(&Level::default(), now);
        assert!(room.world.players[0].vy < 0);
        assert!(!room.world.players[0].jump_held);
        // Many observed IDs coalesce to one edge; even a huge gap is not a queue.
        for seq in 3..100 {
            room.input(id, generation, seq, Input::default(), seq, now);
        }
        room.input(id, generation, 100, Input::default(), u32::MAX, now);
        room.tick(&Level::default(), now);
        assert!(!room.slots[&id].jump_pending);
        for _ in 0..100 {
            room.tick(&Level::default(), now);
        }
        assert!(room.world.players[0].grounded);
        room.input(
            id,
            generation,
            101,
            Input {
                axis: 0,
                jump: true,
            },
            u32::MAX,
            now,
        );
        room.tick(&Level::default(), now);
        assert!(
            room.world.players[0].grounded,
            "duplicate ID must not become a held-state edge"
        );
    }

    #[test]
    fn expired_pending_jump_cannot_be_revived_by_fresh_held_state() {
        let mut room = Room::new();
        let now = Instant::now();
        let a = join(&mut room, None, now);
        room.input(
            a.player,
            a.generation,
            1,
            Input {
                axis: 1,
                jump: true,
            },
            1,
            now,
        );
        room.input(
            a.player,
            a.generation,
            2,
            Input {
                axis: 0,
                jump: true,
            },
            1,
            now + INPUT_LEASE,
        );
        room.tick(&Level::default(), now + INPUT_LEASE);
        assert!(room.world.players[0].grounded);
        assert_eq!(room.world.players[0].vx, 0);
        assert_eq!(room.slots[&a.player].ack, Some(2));
    }

    #[test]
    fn completed_world_still_expires_held_and_pending_input() {
        let mut room = Room::new();
        let now = Instant::now();
        let a = join(&mut room, None, now);
        room.input(
            a.player,
            a.generation,
            1,
            Input {
                axis: 1,
                jump: true,
            },
            1,
            now,
        );
        room.tick(&Level::default(), now);
        assert_ne!(room.world.players[0].vx, 0);
        assert!(room.world.players[0].jump_held);
        room.world.completed = true;
        room.input(
            a.player,
            a.generation,
            2,
            Input {
                axis: -1,
                jump: true,
            },
            2,
            now,
        );
        room.tick(&Level::default(), now + INPUT_LEASE);
        assert!(room.world.completed);
        assert_eq!(room.world.players[0].vx, 0);
        assert!(!room.world.players[0].jump_held);
        assert!(room.slots[&a.player].pending.is_none());
        assert!(!room.slots[&a.player].jump_pending);
        assert_eq!(room.slots[&a.player].held, Input::default());
        assert_eq!(room.slots[&a.player].ack, Some(1));
    }

    #[test]
    fn websocket_read_failures_distinguish_wire_violations_from_transport_loss() {
        use tungstenite::{
            error::{CapacityError, ProtocolError},
            Error as WsError,
        };
        for error in [
            WsError::Capacity(CapacityError::MessageTooLong {
                size: 4097,
                max_size: 4096,
            }),
            WsError::Protocol(ProtocolError::UnmaskedFrameFromClient),
            WsError::Utf8("invalid UTF-8".into()),
            WsError::AttackAttempt,
        ] {
            assert!(read_failure(&axum::Error::new(error)).permanent);
        }
        for error in [
            WsError::Io(std::io::Error::from(std::io::ErrorKind::ConnectionReset)),
            WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake),
            WsError::ConnectionClosed,
        ] {
            assert!(!read_failure(&axum::Error::new(error)).permanent);
        }
        assert!(
            read_failure(&axum::Error::new(std::io::Error::other("unknown wrapper"))).permanent
        );
    }

    #[test]
    fn full_room_resume_fences_old_generation_and_clears_input_and_acks() {
        let mut room = Room::new();
        let now = Instant::now();
        let first = join(&mut room, None, now);
        room.input(
            first.player,
            first.generation,
            42,
            Input {
                axis: 1,
                jump: true,
            },
            1,
            now,
        );
        room.tick(&Level::default(), now);
        for _ in 1..MAX_PLAYERS {
            join(&mut room, None, now);
        }
        let resumed = join(&mut room, Some(&first.token), now);
        assert!(resumed.resumed);
        assert_eq!(resumed.player, first.player);
        assert!(resumed.generation > first.generation);
        assert_eq!(room.slots.len(), MAX_PLAYERS);
        assert_eq!(room.slots[&first.player].ack, None);
        assert!(!room.input(
            first.player,
            first.generation,
            99,
            Input {
                axis: -1,
                jump: true
            },
            99,
            now
        ));
        room.leave(first.player, first.generation, true, now);
        assert!(room.slots[&first.player].connected);
        room.tick(&Level::default(), now);
        assert_eq!(room.world.players[0].vx, 0);
        assert!(room.input(
            first.player,
            resumed.generation,
            1,
            Input::default(),
            0,
            now
        ));
        room.leave(first.player, resumed.generation, false, now);
        assert!(!room.slots[&first.player].connected);
        assert!(room.slots[&first.player].pending.is_none());
        assert!(!room.input(
            first.player,
            resumed.generation,
            2,
            Input::default(),
            1,
            now
        ));
        room.tick(
            &Level::default(),
            now + RESUME_GRACE - Duration::from_millis(1),
        );
        assert_eq!(room.slots.len(), MAX_PLAYERS);
        room.tick(&Level::default(), now + RESUME_GRACE);
        assert_eq!(room.slots.len(), MAX_PLAYERS - 1);
        assert!(matches!(
            room.join(
                watch::channel(String::new()).0,
                watch::channel(false).0,
                Some(&first.token),
                now + RESUME_GRACE
            ),
            Err("invalid_resume_token")
        ));
    }

    #[tokio::test]
    async fn room_bounds_invalid_tokens_and_cleanup() {
        let host = Host::default();
        let make = || (watch::channel(String::new()).0, watch::channel(false).0);
        let (tx, stop) = make();
        assert!(matches!(
            host.join("missing", tx, stop, Some("invalid")).await,
            Err("invalid_resume_token")
        ));
        assert!(host.rooms.lock().await.is_empty());
        for _ in 0..MAX_PLAYERS {
            let (tx, stop) = make();
            host.join("one", tx, stop, None).await.unwrap();
        }
        let (tx, stop) = make();
        assert!(matches!(
            host.join("one", tx, stop, None).await,
            Err("room_full")
        ));
        let (tx, stop) = make();
        assert!(matches!(
            host.join("one", tx, stop, Some("bad")).await,
            Err("invalid_resume_token")
        ));
        for i in 1..MAX_ROOMS {
            let (tx, stop) = make();
            host.join(&format!("room-{i}"), tx, stop, None)
                .await
                .unwrap();
        }
        let (tx, stop) = make();
        assert!(matches!(
            host.join("extra", tx, stop, None).await,
            Err("server_full")
        ));
        let generation = host.rooms.lock().await["room-1"].slots[&1].generation;
        host.leave("room-1", 1, generation, true).await;
        assert!(!host.rooms.lock().await.contains_key("room-1"));
    }

    #[test]
    fn disconnected_participants_cannot_trigger_completion() {
        let now = Instant::now();
        let mut room = Room::new();
        let a = join(&mut room, None, now);
        let b = join(&mut room, None, now);
        room.world.key_collected = true;
        room.world.players[0].x = 22 * sim_core::UNIT;
        room.world.players[1].x = 23 * sim_core::UNIT - sim_core::PLAYER_WIDTH;
        room.leave(a.player, a.generation, false, now);
        room.tick(&Level::default(), now);
        assert!(!room.world.completed);
        join(&mut room, Some(&a.token), now);
        room.tick(&Level::default(), now);
        assert!(room.world.completed);
        assert!(room.slots[&b.player].connected);
    }

    #[test]
    fn latest_mailbox_replaces_old_snapshots_and_replacement_signals_stop() {
        let (tx, mut rx) = watch::channel(String::new());
        for i in 0..100 {
            tx.send_replace(i.to_string());
        }
        assert_eq!(*rx.borrow_and_update(), "99");
        let (stop, old) = watch::channel(false);
        let mut room = Room::new();
        let now = Instant::now();
        let a = room.join(tx, stop, None, now).unwrap();
        join(&mut room, Some(&a.token), now);
        assert!(*old.borrow());
    }

    #[tokio::test]
    async fn blocked_writer_does_not_block_input_reader() {
        let host = Host::default();
        let (tx, rx) = watch::channel(String::new());
        let a = host
            .join("test", tx, watch::channel(false).0, None)
            .await
            .unwrap();
        let stream = futures_util::stream::iter([
            Ok(Message::Text(
                r#"{"type":"input","seq":1,"revision":1,"input":{"axis":1,"jump":false},"jump_id":0}"#.into(),
            )),
            Ok(Message::Close(Some(axum::extract::ws::CloseFrame {
                code: 1000,
                reason: "".into(),
            }))),
        ])
        .then(|message| async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            message
        });
        tokio::pin!(stream);
        let sink = futures_util::sink::unfold((), |(), _: Message| {
            std::future::pending::<Result<(), std::io::Error>>()
        });
        tokio::pin!(sink);
        let outcome = tokio::select! {
            outcome = read_inputs(&mut stream, &host, "test", a.player, a.generation) => outcome,
            _ = write_snapshots(&mut sink, rx, broadcast::channel(4).1, ["welcome".into(), "snapshot".into()], false) => panic!("writer unexpectedly finished"),
        };
        assert!(outcome.permanent);
        assert_eq!(
            host.rooms.lock().await["test"].slots[&a.player].latest_seq,
            1
        );
    }

    #[tokio::test(start_paused = true)]
    async fn ticker_cleans_last_graced_slot_and_empty_room() {
        let host = Host::default();
        let a = host
            .join(
                "test",
                watch::channel(String::new()).0,
                watch::channel(false).0,
                None,
            )
            .await
            .unwrap();
        host.leave("test", a.player, a.generation, false).await;
        let ticker_host = host.clone();
        let ticker = tokio::spawn(async move {
            ticker_host.run_ticks().await;
        });
        tokio::task::yield_now().await;
        tokio::time::advance(RESUME_GRACE + Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert!(host.rooms.lock().await.is_empty());
        ticker.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn blocked_writer_is_bounded_by_timeout() {
        let (_tx, rx) = watch::channel(String::new());
        let sink = futures_util::sink::unfold((), |(), _: Message| {
            std::future::pending::<Result<(), std::io::Error>>()
        });
        tokio::pin!(sink);
        let start = Instant::now();
        write_snapshots(&mut sink, rx, broadcast::channel(4).1, ["welcome".into(), "snapshot".into()], false).await;
        assert_eq!(start.elapsed(), IO_TIMEOUT);
    }

    #[tokio::test(start_paused = true)]
    async fn session_stop_cancels_blocked_writer_and_reader_and_cleans_up_before_close_timeout() {
        use std::sync::atomic::AtomicBool;
        let host = Host::default();
        let (tx, rx) = watch::channel(String::new());
        let (stop, stopped) = watch::channel(false);
        let a = host.join("room", tx, stop.clone(), None).await.unwrap();
        {
            let mut rooms = host.rooms.lock().await;
            let room = rooms.get_mut("room").unwrap();
            room.input(
                a.player,
                a.generation,
                1,
                Input {
                    axis: 1,
                    jump: true,
                },
                1,
                Instant::now(),
            );
            room.tick(&Level::default(), Instant::now());
        }
        let (incoming, messages) = tokio::sync::mpsc::channel(1);
        let writer_started = Arc::new(AtomicBool::new(false));
        let started = writer_started.clone();
        let session_host = host.clone();
        let task = tokio::spawn(async move {
            let sink = futures_util::sink::unfold((), move |(), _: Message| {
                started.store(true, Ordering::Relaxed);
                std::future::pending::<Result<(), std::io::Error>>()
            });
            let stream = futures_util::stream::unfold(messages, |mut messages| async move {
                messages.recv().await.map(|message| (Ok(message), messages))
            });
            tokio::pin!(sink, stream);
            Connection {
                host: &session_host,
                room: "room",
                player: a.player,
                generation: a.generation,
            }
            .run(
                &mut sink,
                &mut stream,
                Writer {
                    rx,
                    stopped,
                    chat: broadcast::channel(4).1,
                    initial: ["welcome".into(), "snapshot".into()],
                    patches: false,
                },
            )
            .await;
        });
        tokio::task::yield_now().await;
        assert!(writer_started.load(Ordering::Relaxed));
        stop.send_replace(true);
        tokio::task::yield_now().await;
        // The final close is blocked too, but cleanup cannot wait for its timeout.
        assert!(!task.is_finished());
        incoming
            .send(Message::Text(
                r#"{"type":"input","seq":2,"revision":1,"jump_id":2,"input":{"axis":1,"jump":true}}"#.into(),
            ))
            .await
            .unwrap();
        tokio::task::yield_now().await;
        {
            let rooms = host.rooms.lock().await;
            let room = &rooms["room"];
            assert!(!room.slots[&a.player].connected);
            assert_eq!(room.slots[&a.player].latest_seq, 1);
            assert!(room.slots[&a.player].pending.is_none());
            assert!(!room.slots[&a.player].jump_pending);
            assert_eq!(room.world.players[0].vx, 0);
        }
        tokio::time::advance(IO_TIMEOUT).await;
        task.await.unwrap();
        let resumed = host
            .join(
                "room",
                watch::channel(String::new()).0,
                watch::channel(false).0,
                Some(&a.token),
            )
            .await
            .unwrap();
        assert!(resumed.resumed);
        assert_eq!(resumed.player, a.player);
    }

    #[tokio::test]
    async fn stale_socket_cannot_remove_or_input_into_recreated_room() {
        let host = Host::default();
        let make = || (watch::channel(String::new()).0, watch::channel(false).0);
        let (tx, stop) = make();
        let old = host.join("room", tx, stop, None).await.unwrap();
        let (tx, stop) = make();
        let replacement = host.join("room", tx, stop, Some(&old.token)).await.unwrap();
        host.leave("room", replacement.player, replacement.generation, true)
            .await;
        let (tx, stop) = make();
        let new = host.join("room", tx, stop, None).await.unwrap();
        assert_eq!(old.player, new.player); // Player IDs may restart, generations may not.
        host.leave("room", old.player, old.generation, true).await;
        let mut rooms = host.rooms.lock().await;
        let room = rooms.get_mut("room").unwrap();
        assert!(!room.input(
            old.player,
            old.generation,
            1,
            Input::default(),
            1,
            Instant::now()
        ));
        assert!(room.slots[&new.player].connected);
    }

    #[tokio::test(start_paused = true)]
    async fn duplicate_sequences_do_not_extend_idle_watchdog() {
        let host = Host::default();
        let joined = host
            .join(
                "room",
                watch::channel(String::new()).0,
                watch::channel(false).0,
                None,
            )
            .await
            .unwrap();
        let stream = futures_util::stream::unfold((), |()| async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Some((
                Ok(Message::Text(
                    r#"{"type":"input","seq":1,"revision":1,"jump_id":0,"input":{"axis":1,"jump":false}}"#
                        .into(),
                )),
                (),
            ))
        });
        tokio::pin!(stream);
        let start = Instant::now();
        let outcome =
            read_inputs(&mut stream, &host, "room", joined.player, joined.generation).await;
        assert_eq!(outcome.error, Some("idle_timeout"));
        assert!(start.elapsed() <= Duration::from_secs(32));
    }

    #[tokio::test]
    async fn connection_headroom_is_bounded_but_not_exhausted_by_all_player_slots() {
        let host = Host::default();
        let _players = host
            .connections
            .clone()
            .try_acquire_many_owned((MAX_ROOMS * MAX_PLAYERS) as u32)
            .unwrap();
        assert_eq!(host.connections.available_permits(), 64);
        let _headroom = host.connections.clone().try_acquire_many_owned(64).unwrap();
        assert!(host.connections.clone().try_acquire_owned().is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn chat_flood_does_not_exhaust_input_budget() {
        let host = Host::default();
        let a = host
            .join(
                "test",
                watch::channel(String::new()).0,
                watch::channel(false).0,
                None,
            )
            .await
            .unwrap();
        // More chat in one second than the whole gameplay budget would tolerate.
        let mut messages: Vec<Result<Message, axum::Error>> = (0..INPUTS_PER_SECOND + 1)
            .map(|_| Ok(Message::Text(r#"{"type":"chat","preset":0}"#.into())))
            .collect();
        messages.push(Ok(Message::Text(
            r#"{"type":"input","seq":1,"revision":1,"input":{"axis":1,"jump":false},"jump_id":0}"#
                .into(),
        )));
        messages.push(Ok(Message::Close(Some(axum::extract::ws::CloseFrame {
            code: 1000,
            reason: "".into(),
        }))));
        let stream = futures_util::stream::iter(messages);
        tokio::pin!(stream);
        let outcome = read_inputs(&mut stream, &host, "test", a.player, a.generation).await;
        assert_eq!(outcome.error, None);
        assert!(outcome.permanent);
        assert_eq!(
            host.rooms.lock().await["test"].slots[&a.player].latest_seq,
            1
        );
    }

    #[tokio::test(start_paused = true)]
    async fn chat_flood_is_dropped_not_fatal() {
        let host = Host::default();
        let a = host
            .join(
                "test",
                watch::channel(String::new()).0,
                watch::channel(false).0,
                None,
            )
            .await
            .unwrap();
        let mut messages: Vec<Result<Message, axum::Error>> = (0..CHAT_FRAMES_PER_SECOND + 1)
            .map(|_| Ok(Message::Text(r#"{"type":"chat","preset":0}"#.into())))
            .collect();
        messages.push(Ok(Message::Text(
            r#"{"type":"input","seq":1,"revision":1,"input":{"axis":1,"jump":false},"jump_id":0}"#
                .into(),
        )));
        messages.push(Ok(Message::Close(Some(axum::extract::ws::CloseFrame {
            code: 1000,
            reason: "".into(),
        }))));
        let stream = futures_util::stream::iter(messages);
        tokio::pin!(stream);
        // Chat over budget is dropped frame by frame, never answered with rate_limited.
        let outcome = read_inputs(&mut stream, &host, "test", a.player, a.generation).await;
        assert_eq!(outcome.error, None);
        assert!(outcome.permanent);
        assert_eq!(
            host.rooms.lock().await["test"].slots[&a.player].latest_seq,
            1
        );
    }

    #[test]
    fn restart_requires_the_controller() {
        let now = Instant::now();
        let mut room = Room::new();
        let owner = join(&mut room, None, now);
        let peer = join(&mut room, None, now);
        // Slots are ID-ordered, so the lowest connected player owns the round.
        assert_eq!(room.controller(), Some(owner.player));
        assert!(!room.command(
            peer.player,
            peer.generation,
            ClientMessage::Restart {
                seq: 1,
                revision: 1,
            },
            now,
        ));
        assert_eq!(room.revision, 1);
        assert_eq!(room.level.number, 1);
        assert!(room.command(
            owner.player,
            owner.generation,
            ClientMessage::Restart {
                seq: 1,
                revision: 1,
            },
            now,
        ));
        assert_eq!(room.revision, 2);
        // Restart re-runs the current stage; only Next advances the campaign.
        assert_eq!(room.level.number, 1);
    }

    #[test]
    fn restart_bumps_revision_and_clears_round_state() {
        let now = Instant::now();
        let mut room = Room::new();
        let owner = join(&mut room, None, now);
        let peer = join(&mut room, None, now);
        let spawns: Vec<i32> = room.world.players.iter().map(|p| p.x).collect();
        for joined in [&owner, &peer] {
            assert!(room.command(
                joined.player,
                joined.generation,
                ClientMessage::Input {
                    seq: 1,
                    input: Input {
                        axis: 1,
                        jump: false,
                    },
                    jump_id: 0,
                    revision: 1,
                },
                now,
            ));
        }
        let level = room.level.clone();
        room.tick(&level, now);
        assert_eq!(room.slots[&owner.player].ack, Some(1));
        assert_eq!(room.slots[&peer.player].ack, Some(1));
        assert_ne!(room.world.players[0].x, spawns[0]);
        assert_ne!(room.world.players[1].x, spawns[1]);
        // Round state that a restart must wipe before the next attempt.
        room.world.completed = true;
        room.world.key_collected = true;
        let tick = room.world.tick;
        let revision = room.revision;
        assert!(room.command(
            owner.player,
            owner.generation,
            ClientMessage::Restart {
                seq: 2,
                revision,
            },
            now + Duration::from_millis(1),
        ));
        assert_eq!(room.revision, revision + 1);
        assert_eq!(room.level.number, 1);
        // Network time survives; the new round starts at the current tick.
        assert_eq!(room.world.tick, tick);
        assert_eq!(room.world.level_start_tick, room.world.tick);
        assert!(!room.world.completed);
        assert!(!room.world.key_collected);
        // Everyone is back at spawn with no carried momentum.
        assert_eq!(
            room.world.players.iter().map(|p| p.x).collect::<Vec<_>>(),
            spawns
        );
        assert!(room
            .world
            .players
            .iter()
            .all(|p| p.vx == 0 && p.vy == 0));
        // Old-round acks are gone; the controller keeps only its restart ack.
        assert_eq!(room.slots[&owner.player].ack, Some(2));
        assert_eq!(room.slots[&peer.player].ack, None);
        assert!(room.slots.values().all(|slot| {
            slot.pending.is_none()
                && slot.held == Input::default()
                && slot.needs_jump_baseline
        }));
        // Input from the abandoned revision never re-enters the new round.
        assert!(!room.command(
            owner.player,
            owner.generation,
            ClientMessage::Input {
                seq: 3,
                input: Input {
                    axis: -1,
                    jump: true,
                },
                jump_id: 3,
                revision,
            },
            now + Duration::from_millis(2),
        ));
    }

    #[test]
    fn next_requires_authoritative_completion() {
        let now = Instant::now();
        let mut room = Room::new();
        let owner = join(&mut room, None, now);
        let revision = room.revision;
        // Only server-observed completion may open the next stage.
        assert!(!room.command(
            owner.player,
            owner.generation,
            ClientMessage::Next {
                seq: 1,
                revision,
            },
            now,
        ));
        assert_eq!(room.revision, revision);
        assert_eq!(room.level.number, 1);
        room.world.completed = true;
        assert!(room.command(
            owner.player,
            owner.generation,
            ClientMessage::Next {
                seq: 2,
                revision,
            },
            now,
        ));
        assert_eq!(room.revision, revision + 1);
        assert_eq!(room.level.number, 2);
        assert!(!room.world.completed);
    }

    #[test]
    fn next_is_refused_at_the_last_stage() {
        let now = Instant::now();
        let mut room = Room::new();
        let owner = join(&mut room, None, now);
        room.level = campaign::level(campaign::LEVEL_COUNT).unwrap();
        room.world = World::for_level(&room.level);
        room.world.completed = true;
        let revision = room.revision;
        assert!(!room.command(
            owner.player,
            owner.generation,
            ClientMessage::Next {
                seq: 1,
                revision,
            },
            now,
        ));
        assert_eq!(room.revision, revision);
        assert_eq!(room.level.number, campaign::LEVEL_COUNT);
        assert!(room.world.completed);
    }

    #[test]
    fn old_revision_input_cannot_affect_the_new_round() {
        let now = Instant::now();
        let mut room = Room::new();
        let owner = join(&mut room, None, now);
        let peer = join(&mut room, None, now);
        let revision = room.revision;
        assert!(room.command(
            owner.player,
            owner.generation,
            ClientMessage::Restart {
                seq: 1,
                revision,
            },
            now,
        ));
        let level = room.level.clone();
        let spawns: Vec<i32> = room.world.players.iter().map(|p| p.x).collect();
        // Both players still speak the abandoned revision, with fresh sequences.
        assert!(!room.command(
            owner.player,
            owner.generation,
            ClientMessage::Input {
                seq: 2,
                input: Input {
                    axis: 1,
                    jump: true,
                },
                jump_id: 7,
                revision,
            },
            now + Duration::from_millis(1),
        ));
        assert!(!room.command(
            peer.player,
            peer.generation,
            ClientMessage::Input {
                seq: 1,
                input: Input {
                    axis: 1,
                    jump: true,
                },
                jump_id: 7,
                revision,
            },
            now + Duration::from_millis(1),
        ));
        assert_eq!(room.slots[&owner.player].latest_seq, 1);
        assert_eq!(room.slots[&peer.player].latest_seq, 0);
        assert!(room.slots.values().all(|slot| {
            slot.pending.is_none() && !slot.jump_pending && slot.jump_id == 0
        }));
        room.tick(&level, now + Duration::from_millis(1));
        assert_eq!(
            room.world.players.iter().map(|p| p.x).collect::<Vec<_>>(),
            spawns
        );
        // The same payload on the current revision lands, so the revision is the fence.
        assert!(room.command(
            owner.player,
            owner.generation,
            ClientMessage::Input {
                seq: 2,
                input: Input {
                    axis: 1,
                    jump: true,
                },
                jump_id: 7,
                revision: revision + 1,
            },
            now + Duration::from_millis(2),
        ));
        assert_eq!(room.slots[&owner.player].latest_seq, 2);
        assert_eq!(
            room.slots[&owner.player].pending,
            Some((2, Input::default()))
        );
    }
}
