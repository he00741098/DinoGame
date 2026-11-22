// Diagnostic presentation only: no duplicate simulation or client-supplied positions.
const canvas = document.querySelector('#game');
const ctx = canvas.getContext('2d');
const status = document.querySelector('#status');
const form = document.querySelector('#join');
const roomField = document.querySelector('#room');
const restartButton = document.querySelector('#restart');
const nextButton = document.querySelector('#next');
const networkStatistics = new NetworkStatistics();
const motion = window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches
  ? new MotionTimeline({ extrapolationCapMs: 0, smoothingMs: 0 })
  : new MotionTimeline();
let lastInputSignature, lastInputSent = -Infinity, lastStatisticsUpdate = -Infinity;
function transmit(current, message) {
  const text = JSON.stringify(message);
  current.send(text);
  networkStatistics.sent(text, message.seq, performance.now());
}
const colors = ['#b9f479', '#73d9ff', '#ffbd80', '#dbadff'];
const keys = new Set();
const touches = new Map();
let socket, player, level, world, seq = 0;
let jumpId = 0, committedJumpId = 0, resume;
let revision, controller, campaignComplete = false;
let inputReady = false, baselineSeq;
let lastFinalHeartbeat = -Infinity;
let lastSnapshot = 0;
let ackTargetSeq, ackSince, stalePaused = false, hasAppliedAck = false;
let retryTimer, retryAttempts = 0, retryAllowed = false;
const unit = 1024;
const arrivals = new Map();
let reviewing = false, reviewGeneration = 0, reviewFrames = [], reviewPosition = 0, reviewLastTime = 0, reviewPlaying = false;
let reviewFrameMilliseconds = 50;
const chatBubbles = new Map();
const chatHistory = [];
const queryRoom = new URL(location.href).searchParams.get('room');
if (queryRoom && /^[A-Za-z0-9-]{3,16}$/.test(queryRoom)) roomField.value = queryRoom;

function clearInput() {
  keys.clear(); touches.clear();
  // Cancel only edges never sent; transmitted IDs must never regress within a session.
  jumpId = committedJumpId;
  sendInput();
}
function actionHeld(action) {
  const bindings = { left: ['ArrowLeft', 'KeyA'], right: ['ArrowRight', 'KeyD'], jump: ['Space', 'ArrowUp', 'KeyW'] };
  return bindings[action].some(key => keys.has(key)) || [...touches.values()].includes(action);
}
function updateInput(change) {
  if (!inputReady || campaignComplete) return;
  const wasJump = actionHeld('jump');
  change();
  const isJump = actionHeld('jump');
  if (!wasJump && isJump && !document.hidden && player !== undefined) {
    if (jumpId === 0xffffffff) { socket.close(1000); return; }
    ++jumpId;
  }
  // Capture even a down/up pair that occurs entirely between timer callbacks.
  if (wasJump !== isJump) sendInput();
}
function setStatus(text) { status.textContent = text; }
// Screen readers get meaningful state changes only; #status itself is a
// per-snapshot diagnostic stream and must never be a live region.
const announceRegion = document.querySelector('#announce');
let lastAnnouncement = '';
function announce(text) {
  if (!announceRegion || text === lastAnnouncement) return;
  lastAnnouncement = text;
  announceRegion.textContent = text;
}
function updateControls() {
  const connected = player !== undefined && socket?.readyState === WebSocket.OPEN;
  const owner = connected && inputReady && controller === player;
  for (const button of document.querySelectorAll('[data-action]')) button.disabled = !connected || !inputReady || campaignComplete;
  restartButton.disabled = !owner;
  nextButton.disabled = true;
  nextButton.hidden = true; // Automatic authoritative progression replaces manual Next.
}
function canSend() {
  if (!socket || socket.readyState !== WebSocket.OPEN || player === undefined || revision === undefined) return false;
  // All commands share the same bounded stream and sequence counter.
  if (socket.bufferedAmount > 8192 || seq === 0xffffffff) { socket.close(4001); return false; }
  return true;
}
function offered(sequence) {
  if (ackTargetSeq === undefined) { ackTargetSeq = sequence; ackSince = performance.now(); }
}
for (const [button, type] of [[restartButton, 'restart'], [nextButton, 'next']]) {
  button.addEventListener('click', () => {
    updateControls();
    if (!button.disabled && canSend()) {
      campaignComplete = false; lastSnapshot = performance.now();
      offered(++seq);
      transmit(socket, { type, seq, revision });
    }
  });
}
updateControls();

function connectRoom(room, automatic = false) {
  motion.reset();
  arrivals.clear();
  reviewing = false; ++reviewGeneration; reviewFrames = [];
  clearTimeout(retryTimer); retryTimer = undefined;
  retryAllowed = true;
  if (!automatic) retryAttempts = 0;
  clearInput();
  const endpoint = `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/ws`;
  const token = resume?.room === room && resume.endpoint === endpoint ? resume.token : undefined;
  const previous = socket;
  socket = undefined;
  // Browser APIs forbid sending reserved code 1001; 4001 means recoverable reconnect.
  if (previous) previous.close(token ? 4001 : 1000);
  if (!token) resume = undefined;
  world = level = player = revision = controller = undefined;
  campaignComplete = false;
  lastFinalHeartbeat = -Infinity;
  inputReady = false; baselineSeq = undefined;
  ackTargetSeq = ackSince = undefined; stalePaused = false; hasAppliedAck = false;
  lastSnapshot = performance.now();
  networkStatistics.begin(lastSnapshot);
  lastInputSignature = undefined; lastInputSent = -Infinity;
  chatBubbles.clear();
  updateControls();
  const url = new URL(location.href);
  url.searchParams.set('room', room);
  history.replaceState(null, '', url);
  const current = new WebSocket(endpoint);
  const wireDecoder = new SnapshotWireDecoder();
  let rejected = false;
  socket = current;
  document.querySelector('#connect').textContent = 'Reconnect';
  setStatus('Connecting…');
  current.addEventListener('open', () => {
    if (socket !== current) return;
    transmit(current, { type: 'hello', protocol: 3, game: 'coop-sandbox', ruleset: 'coop-campaign-3', room, patches: true,
      ...(token ? { resume_token: token } : {}) });
  });
  current.addEventListener('message', event => {
    if (socket !== current) return;
    networkStatistics.received(event.data);
    let message;
    try {
      if (new TextEncoder().encode(event.data).length > 65536) throw new Error('Oversized frame');
      message = wireDecoder.decode(JSON.parse(event.data));
    }
    catch { current.close(4001); return; }
    if (message.type === 'welcome') {
      player = message.player;
      seq = jumpId = committedJumpId = 0;
      keys.clear(); touches.clear();
      revision = message.revision;
      controller = message.controller;
      world = undefined;
      campaignComplete = false;
      inputReady = true; baselineSeq = undefined;
      resume = { endpoint, room, token: message.resume_token };
      level = message.level;
      lastSnapshot = performance.now();
      updateControls();
      setStatus(`Room ${room} · You are P${player} · Level ${level.number}/28: ${level.title} · ${level.required_players ?? 2} players needed · ${level.hint}`);
      announce(`Joined room ${room} as P${player} — Level ${level.number} of 28: ${level.title}`);
    } else if (message.type === 'snapshot') {
      if (player === undefined || revision === undefined) return;
      if (message.revision < revision || (world && message.world.tick < world.tick)) {
        setStatus(`Ignored stale snapshot · revision ${message.revision} · tick ${message.world.tick}`);
        return;
      }
      const changed = message.revision > revision;
      const fresh = !world || message.world.tick > world.tick || changed;
      if (!fresh && stalePaused) return;
      const previousLevelNumber = level?.number;
      const wasCampaignComplete = campaignComplete;
      const wasTeamComplete = world?.completed;
      const wasStalled = stalePaused;
      revision = message.revision;
      level = message.level;
      if (changed || world?.deaths !== message.world.deaths) motion.reset();
      world = message.world;
      motion.append(world.tick / 60, performance.now() / 1000, world.players.map(p => ({ ...p, vx: (p.vx ?? 0) * 60, vy: (p.vy ?? 0) * 60 })));
      controller = message.controller;
      campaignComplete = message.campaign_complete;
      if (changed) {
        arrivals.clear();
        ackTargetSeq = ackSince = undefined; hasAppliedAck = false;
        inputReady = false; baselineSeq = undefined;
        clearInput();
      }
      const ack = message.acks.find(ack => ack.player === player)?.seq;
      networkStatistics.snapshot(ack, performance.now());
      if (ackTargetSeq !== undefined && ack >= ackTargetSeq) { ackTargetSeq = ackSince = undefined; }
      if (baselineSeq !== undefined && ack >= baselineSeq) {
        inputReady = true; baselineSeq = undefined;
        hasAppliedAck = true;
      }
      if (fresh) {
        stalePaused = false;
        lastSnapshot = performance.now();
      }
      if (hasAppliedAck) retryAttempts = 0;
      updateControls();
      setStatus(`Room ${room} · P${player} · Level ${level.number}/28: ${level.title} · ${level.required_players ?? 2} players needed · ${level.hint} · ` +
        `${world.players.length}/4 players · tick ${world.tick} · revision ${revision} · ack ${ack ?? 'pending'} · deaths ${world.deaths}` +
        `${inputReady ? '' : ' · Waiting for neutral input acknowledgment'}` +
        `${world.key_collected ? ` · key collected · exit ${world.players.filter(p => atExit(p)).length}/${world.players.length} (enter to wait)` : ''}${campaignComplete ? ' · Campaign complete! All 28 levels cleared.' : world.completed ? ' · Level complete! Next stage shortly…' : ''}`);
      if (changed) announce(`Revision ${revision}`);
      if (previousLevelNumber !== level.number) announce(`Level ${level.number} of 28: ${level.title}`);
      if (campaignComplete && !wasCampaignComplete) announce('Campaign complete. All 28 levels cleared.');
      if (world.completed && !wasTeamComplete) announce('Level complete. Next stage shortly.');
      if (wasStalled && fresh) announce('Connection recovered.');
    } else if (message.type === 'chat') {
      chatBubbles.set(message.player, { text: message.text, until: performance.now() + 3500 });
      chatHistory.push(`P${message.player}: ${message.text}`);
      if (chatHistory.length > 3) chatHistory.shift();
      const log = document.querySelector('#chatlog');
      if (log) log.textContent = chatHistory.join(' · ');
    } else if (message.type === 'error') {
      if (message.code === 'invalid_resume_token') resume = undefined;
      rejected = true;
      retryAllowed = false;
      setStatus(`Server: ${message.message}`);
      announce(`Server: ${message.message}`);
      player = undefined;
      updateControls();
    }
  });
  current.addEventListener('close', event => {
    if (socket === current) {
      if (event.code === 1000) { resume = undefined; retryAllowed = false; }
      if (!rejected) {
        setStatus('Disconnected — reconnect within 15 seconds to resume.');
        announce('Disconnected — reconnect within 15 seconds to resume.');
      }
      player = undefined;
      updateControls();
      clearInput();
      if (!rejected && retryAllowed) {
        const delay = Math.min(8000, 500 * 2 ** retryAttempts);
        retryAttempts = Math.min(4, retryAttempts + 1);
        retryTimer = setTimeout(() => {
          if (socket === current && retryAllowed) connectRoom(room, true);
        }, delay);
      }
    }
  });
  current.addEventListener('error', () => {
    if (socket === current) {
      setStatus('Connection failed. Check that the development server is running.');
      announce('Connection failed. Check that the development server is running.');
    }
  });
  // Mouse/touch submit leaves focus on the button, which otherwise eats game keys.
  if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
}
form.addEventListener('submit', event => {
  event.preventDefault();
  connectRoom(roomField.value);
});

const movementKeys = new Set(['ArrowLeft', 'ArrowRight', 'ArrowUp', 'KeyA', 'KeyD', 'KeyW', 'Space']);
window.addEventListener('keydown', event => {
  if (event.target instanceof HTMLInputElement) return;
  if (event.target instanceof HTMLButtonElement && ['Space', 'Enter'].includes(event.code)) return;
  if (movementKeys.has(event.code)) { event.preventDefault(); updateInput(() => keys.add(event.code)); }
});
window.addEventListener('keyup', event => updateInput(() => keys.delete(event.code)));
window.addEventListener('blur', clearInput);
document.addEventListener('visibilitychange', clearInput);
for (const button of document.querySelectorAll('[data-action]')) {
  const keyboardPointer = `keyboard-${button.dataset.action}`;
  button.addEventListener('keydown', event => {
    if (['Space', 'Enter'].includes(event.code)) {
      event.preventDefault();
      updateInput(() => touches.set(keyboardPointer, button.dataset.action));
    }
  });
  button.addEventListener('keyup', event => {
    if (['Space', 'Enter'].includes(event.code)) {
      event.preventDefault();
      updateInput(() => touches.delete(keyboardPointer));
    }
  });
  button.addEventListener('blur', () => updateInput(() => touches.delete(keyboardPointer)));
  button.addEventListener('pointerdown', event => {
    event.preventDefault();
    button.setPointerCapture(event.pointerId);
    updateInput(() => touches.set(event.pointerId, button.dataset.action));
  });
  for (const kind of ['pointerup', 'pointercancel', 'lostpointercapture']) {
    button.addEventListener(kind, event => updateInput(() => touches.delete(event.pointerId)));
  }
}
function sendInput() {
  if (!canSend()) return;
  if (campaignComplete) {
    if (performance.now() - lastFinalHeartbeat < 10000) return;
    lastFinalHeartbeat = performance.now();
  }
  const hidden = document.hidden || !inputReady || campaignComplete;
  if (hidden) jumpId = committedJumpId;
  const input = {
    axis: hidden ? 0 : Number(actionHeld('right')) - Number(actionHeld('left')),
    jump: !hidden && actionHeld('jump'),
  };
  const signature = `${revision}:${jumpId}:${input.axis}:${input.jump}`;
  const now = performance.now();
  if (signature === lastInputSignature && now - lastInputSent < 100) return;
  lastInputSignature = signature; lastInputSent = now;
  offered(++seq);
  transmit(socket, { type: 'input', seq, revision, jump_id: jumpId, input });
  committedJumpId = jumpId;
  if ((!inputReady || !hasAppliedAck) && baselineSeq === undefined) {
    baselineSeq = seq;
  }
}
function serviceConnection() {
  if (reviewing) return;
  const statistics = document.querySelector('#networkStatistics');
  if (statistics && performance.now() - lastStatisticsUpdate >= 1000) {
    statistics.textContent = networkStatistics.summary(performance.now());
    lastStatisticsUpdate = performance.now();
  }
  if (socket && [WebSocket.CONNECTING, WebSocket.OPEN].includes(socket.readyState) && retryAllowed) {
    const now = performance.now(), age = now - lastSnapshot;
    if ((ackSince !== undefined && now - ackSince >= 3000) || age >= (campaignComplete ? 15000 : 5000)) {
      inputReady = false; keys.clear(); touches.clear(); jumpId = committedJumpId;
      updateControls();
      setStatus('Connection stalled — recovering the session…');
      announce('Connection stalled — recovering the session…');
      socket.close(4001);
      return;
    }
    if (player !== undefined && !campaignComplete && age >= 750 && !stalePaused) {
      stalePaused = true; inputReady = false;
      baselineSeq = undefined;
      clearInput();
      updateControls();
      setStatus('Waiting for server — controls released…');
      announce('Waiting for server — controls released…');
    }
  }
  // Keep sending neutral heartbeats while waiting for an acknowledgment. A
  // single baseline can otherwise expire at the server during a stalled tick.
  sendInput();
}
setInterval(serviceConnection, 1000 / 60);

function box(rect, color) {
  ctx.fillStyle = color;
  ctx.fillRect(rect.x / unit * 50, rect.y / unit * 50, rect.w / unit * 50, rect.h / unit * 50);
}
function atExit(p) {
  return p.x < level.exit.x + level.exit.w && p.x + 720 > level.exit.x &&
    p.y < level.exit.y + level.exit.h && p.y + 896 > level.exit.y;
}
function spikes(rect) {
  const teeth = Math.max(1, Math.min(24, Math.floor(rect.w / 350)));
  const scale = 50 / unit;
  ctx.fillStyle = '#ff667f';
  ctx.beginPath();
  ctx.moveTo(rect.x * scale, (rect.y + rect.h) * scale);
  for (let i = 0; i < teeth; i++) {
    ctx.lineTo((rect.x + rect.w * (i + 0.5) / teeth) * scale, rect.y * scale);
    ctx.lineTo((rect.x + rect.w * (i + 1) / teeth) * scale, (rect.y + rect.h) * scale);
  }
  ctx.closePath(); ctx.fill();
}
// Server object rectangles/state are authoritative; these glyphs do not simulate mechanics.
const mechanicStyles = {
  plate: ['#dbb663', 'P'], pressure_gate: ['#df984e', 'PG'], timed_gate: ['#df7171', 'TG'],
  one_way: ['#78b7d9', '↑'], moving_platform: ['#72c6c1', '↔'], conveyor: ['#ad9bdf', '»'],
  ice: ['#b4edff', 'I'], spring: ['#9dec77', '↑↑'], hazard: ['#ff667f', '!'],
  checkpoint: ['#70e0a5', 'C'], crumble: ['#bfa58c', '…'], low_gravity: ['#c794f5', 'G↓'], wind: ['#81cdda', '~'],
  rope: ['#e7c28c', 'ROPE: hold Jump together'], falling_rock: ['#b7bcc9', 'ROCK'],
  team_pad: ['#e87fb5', '👥'],
};
function draw() {
  if (reviewing && reviewFrames.length) {
    const now = performance.now();
    const speed = Number(document.querySelector('#reviewSpeed')?.value ?? 1);
    if (reviewPlaying) reviewPosition = Math.min(reviewFrames.length - 1, reviewPosition + (now - reviewLastTime) / reviewFrameMilliseconds * speed);
    reviewLastTime = now;
    world = reviewFrames[Math.floor(reviewPosition)];
    const seek = document.querySelector('#reviewSeek');
    if (seek) seek.value = String(Math.floor(reviewPosition));
  }
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  if (level && world) {
    const displayPlayers = world.players.map(p => ({ ...p, ...(!reviewing ? motion.position(p.id, performance.now() / 1000, p.id === player) : undefined) }));
    // Catalog stages grow beyond the old 24-tile viewport. Resize the backing
    // canvas to the bounded full level; CSS scales it to the browser's width.
    const width = reviewing ? Math.min(6400, Math.max(800, Math.ceil(level.width / unit * 50))) : 1200;
    const height = reviewing ? Math.min(3200, Math.max(700, Math.ceil(level.height / unit * 50))) : 700;
    if (Number.isFinite(width) && canvas.width !== width) canvas.width = width;
    if (Number.isFinite(height) && canvas.height !== height) canvas.height = height;
    const focus = displayPlayers.find(p => p.id === player) ?? displayPlayers[0] ?? { x: 0, y: level.height };
    const viewWidth = canvas.width / 50 * unit, viewHeight = canvas.height / 50 * unit;
    const left = reviewing ? 0 : Math.max(0, Math.min(level.width - viewWidth, focus.x + 360 - viewWidth / 2));
    const top = reviewing ? 0 : Math.max(0, Math.min(level.height - viewHeight, focus.y + 448 - viewHeight / 2));
    ctx.save(); ctx.translate(-left / unit * 50, -top / unit * 50);
    for (const solid of level.solids) box(solid, '#456078');
    if (!world.key_collected) box(level.gate, '#806b9d');
    box(level.exit, world.key_collected ? '#76e3bb' : '#59687b');
    const mechanics = new Map(level.mechanics.map(mechanic => [mechanic.id, mechanic.kind]));
    for (const object of world.objects) {
      const kind = mechanics.get(object.id);
      const [color, glyph] = mechanicStyles[kind] ?? ['#ffffff', '?'];
      if (kind === 'falling_rock' && object.timer < 60) {
        const definition = level.mechanics.find(item => item.id === object.id);
        ctx.globalAlpha = 0.12;
        box({ ...definition.rect, h: definition.rect.h + definition.travel }, '#ff667f');
        ctx.globalAlpha = 1;
        ctx.fillStyle = '#ff667f'; ctx.font = 'bold 14px system-ui';
        ctx.fillText('⚠ ROCK', definition.rect.x / unit * 50, (definition.rect.y + definition.travel) / unit * 50 - 6);
      }
      const zone = ['low_gravity', 'wind', 'checkpoint', 'rope'].includes(kind);
      ctx.globalAlpha = zone ? 0.16 : 1;
      if (kind === 'hazard') spikes(object.rect);
      else box(object.rect, object.active ? color : '#394356');
      ctx.globalAlpha = 1;
      if (kind === 'rope') {
        ctx.fillStyle = object.active ? '#ffe175' : '#977857';
        ctx.fillRect((object.rect.x + object.rect.w / 2) / unit * 50 - 2, object.rect.y / unit * 50, 4, object.rect.h / unit * 50);
      }
      ctx.fillStyle = color; ctx.font = '12px system-ui';
      ctx.fillText(`${kind === 'falling_rock' && object.timer < 60 ? '⚠ ' : ''}${glyph} #${object.id} ${object.active ? 'ON' : 'off'} t:${object.timer}`,
        object.rect.x / unit * 50, object.rect.y / unit * 50 - 3);
      // A pad waiting for a crowd advertises how many players it still needs.
      if (kind === 'team_pad' && !object.active) {
        const definition = level.mechanics.find(item => item.id === object.id);
        const required = Number(definition?.travel);
        const needed = Number.isFinite(required) && required > 1 ? Math.trunc(required) : 2;
        ctx.fillStyle = color; ctx.font = 'bold 12px system-ui';
        ctx.fillText(`×${needed}`, object.rect.x / unit * 50, object.rect.y / unit * 50 - 16);
      }
    }
    // Objectives are drawn LAST over zones/objects: the purple low-G zone used
    // to paint an opaque rectangle over its elevated key.
    if (!world.key_collected) {
      box(level.key, '#ffe175');
      ctx.fillStyle = '#ffffff'; ctx.font = 'bold 14px system-ui';
      ctx.fillText('KEY', level.key.x / unit * 50, level.key.y / unit * 50 - 8);
    }
    if (world.checkpoint) {
      ctx.fillStyle = '#70e0a5'; ctx.font = '12px system-ui';
      ctx.fillText('✓ checkpoint', world.checkpoint.x / unit * 50, world.checkpoint.y / unit * 50 - 16);
    }
    for (const id of arrivals.keys()) if (!world.players.some(p => p.id === id && world.key_collected && atExit(p))) arrivals.delete(id);
    // Drop bubbles whose lifetime elapsed or whose player left the room; otherwise they persist for the page lifetime.
    const now = performance.now();
    for (const [id, bubble] of chatBubbles) {
      if (bubble.until <= now || !world.players.some(p => p.id === id)) chatBubbles.delete(id);
    }
    if ((level.tether_length ?? 0) > 0) {
      ctx.strokeStyle = '#ffe175'; ctx.lineWidth = 2; ctx.beginPath();
      const ordered = [...displayPlayers].sort((a, b) => a.id - b.id);
      for (let index = 1; index < ordered.length; index++) {
        const a = ordered[index - 1], b = ordered[index];
        if ([a, b].some(p => world.key_collected && atExit(world.players.find(auth => auth.id === p.id)))) continue;
        ctx.moveTo((a.x + 360) / unit * 50, (a.y + 448) / unit * 50);
        ctx.lineTo((b.x + 360) / unit * 50, (b.y + 448) / unit * 50);
      }
      ctx.stroke();
    }
    for (const p of displayPlayers) {
      const arrived = world.key_collected && atExit(world.players.find(auth => auth.id === p.id));
      if (arrived && !arrivals.has(p.id)) arrivals.set(p.id, performance.now());
      const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches ?? false;
      const progress = arrived ? reduced ? 1 : Math.min(1, (performance.now() - arrivals.get(p.id)) / 350) : 0;
      const scale = 1 - progress * 0.9;
      const x = p.x + (level.exit.x + level.exit.w / 2 - 360 - p.x) * progress;
      const y = p.y + (level.exit.y + level.exit.h / 2 - 448 - p.y) * progress;
      ctx.globalAlpha = 1 - progress;
      box({ x: x + 360 * (1 - scale), y: y + 448 * (1 - scale), w: 720 * scale, h: 896 * scale }, colors[(p.id - 1) % colors.length]);
      ctx.fillStyle = '#ffffff'; ctx.font = '15px system-ui';
      ctx.fillText(`P${p.id}${p.id === player ? ' (you)' : ''}`, x / unit * 50, y / unit * 50 - 10);
      const bubble = chatBubbles.get(p.id);
      if (bubble && bubble.until > performance.now()) {
        ctx.fillStyle = '#ffe175'; ctx.font = 'bold 13px system-ui';
        ctx.fillText(bubble.text, x / unit * 50, y / unit * 50 - 28);
      }
      ctx.globalAlpha = 1;
    }
    if (arrivals.size) { ctx.fillStyle = '#ffffff'; ctx.fillText(`${arrivals.size}/${world.players.length} ✓`, level.exit.x / unit * 50, level.exit.y / unit * 50 - 12); }
    ctx.restore();
    if (!reviewing) {
      const markers = displayPlayers.filter(p => p.id !== player).map(p => ({ x: p.x + 360, y: p.y + 448, title: `P${p.id}`, color: '#ffffff' }));
      if (!world.key_collected) markers.push({ x: level.key.x, y: level.key.y, title: 'KEY', color: '#ffe175' });
      for (const marker of markers) {
        const x = (marker.x - left) / unit * 50, y = (marker.y - top) / unit * 50;
        if (x >= 20 && x <= canvas.width - 60 && y >= 20 && y <= canvas.height - 20) continue;
        ctx.fillStyle = marker.color; ctx.font = 'bold 14px system-ui';
        const arrow = x < 20 ? '←' : x > canvas.width - 60 ? '→' : y < 20 ? '↑' : '↓';
        ctx.fillText(`${arrow} ${marker.title}`, Math.max(20, Math.min(canvas.width - 70, x)), Math.max(20, Math.min(canvas.height - 20, y)));
      }
    }
    if (player !== undefined && !campaignComplete && performance.now() - lastSnapshot > 1500) {
      ctx.fillStyle = '#ffffff'; ctx.fillText('Waiting for server…', 24, 36);
    }
  }
  requestAnimationFrame(draw);
}
requestAnimationFrame(draw);

async function startReview(number) {
  motion.reset();
  arrivals.clear();
  if (!Number.isInteger(number) || number < 1 || number > 28) return;
  reviewing = true;
  const request = ++reviewGeneration;
  clearTimeout(retryTimer); retryAllowed = false;
  inputReady = false; keys.clear(); touches.clear();
  const previous = socket; socket = undefined;
  if (previous) previous.close(1000);
  resume = undefined; player = controller = undefined; world = level = undefined;
  reviewFrames = []; reviewPlaying = false;
  updateControls();
  setStatus(`Loading level ${number} solution… (not connected to a live room)`);
  try {
    const response = await fetch(`/solutions/${number}`);
    if (!response.ok) throw new Error('Solution unavailable');
    const recording = await response.json();
    if (!reviewing || request !== reviewGeneration) return;
    if (recording.format !== 1 || recording.level.number !== number || recording.frames.length < 2 || recording.frames.length > 2000 ||
        !recording.frames.at(-1).completed || recording.frames.at(-1).deaths !== 0) throw new Error('Invalid solution recording');
    level = recording.level; reviewFrames = recording.frames; reviewPosition = 0;
    reviewFrameMilliseconds = recording.frame_stride / recording.tick_hz * 1000;
    for (const frame of reviewFrames) if (!frame.objects && recording.constant_objects) frame.objects = recording.constant_objects;
    world = reviewFrames[0]; reviewPlaying = true; reviewLastTime = performance.now();
    const seek = document.querySelector('#reviewSeek');
    if (seek) { seek.max = String(reviewFrames.length - 1); seek.value = '0'; }
    setStatus(`SOLUTION REVIEW · ${number}/28: ${level.title} · ${level.hint} · ${recording.provenance}`);
  } catch {
    if (reviewing && request === reviewGeneration) {
      setStatus('Solution could not be loaded. Retry or Join a live room.');
      announce('Solution could not be loaded. Retry or Join a live room.');
    }
  }
}
document.querySelector('#review')?.addEventListener('click', () => startReview(Number(document.querySelector('#reviewStage').value)));
for (const button of document.querySelectorAll('[data-chat]')) {
  button.addEventListener('click', () => {
    if (socket?.readyState === WebSocket.OPEN) transmit(socket, { type: 'chat', preset: Number(button.dataset.chat) });
  });
}
document.querySelector('#reviewPause')?.addEventListener('click', () => { reviewPlaying = !reviewPlaying; reviewLastTime = performance.now(); });
document.querySelector('#reviewSeek')?.addEventListener('input', event => {
  arrivals.clear();
  reviewPosition = Math.max(0, Math.min(reviewFrames.length - 1, Number(event.target.value)));
  reviewPlaying = false;
});
const querySolution = Number(new URL(location.href).searchParams.get('solution'));
if (querySolution >= 1 && querySolution <= 28) startReview(querySolution);
