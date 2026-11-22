// Dependency-free contract tests for the diagnostic shell (not a browser/GPU test).
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';

class Element {
  listeners = new Map();
  textContent = '';
  addEventListener(kind, fn) {
    if (!this.listeners.has(kind)) this.listeners.set(kind, []);
    this.listeners.get(kind).push(fn);
  }
  emit(kind, event = {}) {
    for (const fn of this.listeners.get(kind) ?? []) fn({ preventDefault() {}, ...event });
  }
  blur() { this.blurred = true; }
  setPointerCapture() {}
}
class InputElement extends Element { value = 'playtest'; }
class ButtonElement extends Element { disabled = false; }

const rect = { x: 1024, y: 2048, w: 1024, h: 1024 };
function level(number = 1, mechanics = []) {
  return { number, width: 24576, height: 14336, title: `Stage ${number}`, hint: 'Work together', mechanics, solids: [], gate: rect, exit: rect, key: rect };
}
function snapshot(overrides = {}) {
  return { type: 'snapshot', revision: 1, controller: 1, campaign_complete: false, level: level(), acks: [],
    world: { tick: 10, players: [], objects: [], deaths: 0, checkpoint: null, level_start_tick: 0, completed: false, key_collected: false }, ...overrides };
}

function harness(options = {}) {
  const fields = new Map();
  for (const id of ['join', 'status', 'game']) fields.set(`#${id}`, new Element());
  fields.set('#room', new InputElement());
  fields.set('#connect', new ButtonElement());
  fields.set('#restart', new ButtonElement());
  fields.set('#next', new ButtonElement());
  fields.set('#networkStatistics', new Element());
  fields.set('#chatlog', new Element());
  for (const id of ['review', 'reviewPause']) fields.set(`#${id}`, new ButtonElement());
  for (const id of ['reviewStage', 'reviewSpeed', 'reviewSeek']) { const input = new InputElement(); input.value = '1'; fields.set(`#${id}`, input); }
  const draws = [];
  fields.get('#game').getContext = () => ({ globalAlpha: 1, clearRect() {},
    save() {}, restore() {}, translate(...args) { draws.push(['translate', ...args]); },
    fillRect(...args) { draws.push(['rect', ...args, this.fillStyle, this.globalAlpha]); },
    fillText(...args) { draws.push(['text', ...args, this.globalAlpha]); },
    beginPath() { draws.push(['path']); }, moveTo(...args) { draws.push(['move', ...args]); },
    lineTo(...args) { draws.push(['line', ...args]); }, closePath() {}, fill() {}, stroke() {},
  });
  const controls = ['left', 'right', 'jump'].map(action => {
    const button = new ButtonElement(); button.dataset = { action }; return button;
  });
  const chats = [0, 1, 2, 3, 4, 5].map(preset => {
    const button = new ButtonElement(); button.dataset = { chat: String(preset) }; return button;
  });
  const doc = new Element();
  doc.querySelector = id => fields.get(id);
  doc.querySelectorAll = selector => selector === '[data-chat]' ? chats : controls;
  doc.activeElement = fields.get('#connect');
  doc.hidden = false;
  const window = new Element();
  const sockets = [];
  class Socket extends Element {
    static CONNECTING = 0;
    static OPEN = 1;
    readyState = 0;
    bufferedAmount = 0;
    sent = [];
    constructor(url) { super(); this.url = url; sockets.push(this); }
    send(text) { this.sent.push(JSON.parse(text)); }
    close(code) {
      if (code !== undefined && code !== 1000 && (code < 3000 || code > 4999)) throw new Error('Browser forbids reserved close code');
      this.closeCode = code; this.readyState = 3; this.emit('close', { code: code ?? 1005 });
    }
    open() { this.readyState = 1; this.emit('open'); }
    receive(message) { this.emit('message', { data: JSON.stringify(message) }); }
  }
  let interval, frame, now = 0;
  const timers = new Map();
  let nextTimer = 1;
  const context = vm.createContext({
    document: doc, window, WebSocket: Socket, URL, TextEncoder,
    fetch: options.fetch ?? (async path => ({ ok: true, json: async () => JSON.parse(readFileSync(new URL(`./solutions/${path.split('/').at(-1)}.json`, import.meta.url), 'utf8')) })),
    HTMLInputElement: InputElement, HTMLButtonElement: ButtonElement, HTMLElement: Element,
    location: { href: 'http://localhost:3000/', host: 'localhost:3000', protocol: 'http:' },
    history: { replaceState() {} }, performance: { now: () => now },
    setTimeout(fn, delay) { const id = nextTimer++; timers.set(id, { fn, at: now + delay }); return id; },
    clearTimeout(id) { timers.delete(id); },
    setInterval: fn => { interval = fn; }, requestAnimationFrame(fn) { frame = fn; },
  });
  vm.runInContext(readFileSync(new URL('./network.js', import.meta.url), 'utf8'), context);
  vm.runInContext(readFileSync(new URL('./motion.js', import.meta.url), 'utf8'), context);
  vm.runInContext(readFileSync(new URL('./client.js', import.meta.url), 'utf8'), context);
  const join = () => {
    fields.get('#join').emit('submit');
    const socket = sockets.at(-1);
    socket.open();
    socket.receive({ type: 'welcome', player: 1, revision: 1, controller: 1, level: level(), resume_token: 'memory-only-token', resumed: sockets.length > 1 });
    return socket;
  };
  const key = (kind, code) => window.emit(kind, { code, target: fields.get('#game') });
  const advance = milliseconds => {
    now += milliseconds;
    interval();
    for (const [id, timer] of [...timers]) if (timer.at <= now) { timers.delete(id); timer.fn(); }
  };
  return { fields, doc, window, controls, chats, sockets, join, key, advance, review: number => vm.runInContext(`startReview(${number})`, context), tick: () => interval(), draw: () => { draws.length = 0; frame(); return draws; } };
}

test('long campaign stages use a bounded following viewport', () => {
  const h = harness(); const socket = h.join();
  socket.receive(snapshot({ level: { ...level(20), width: 40960, exit: { x: 38 * 1024, y: 10240, w: 1024, h: 2048 } } }));
  h.draw();
  assert.equal(h.fields.get('#game').width, 1200);
  assert.equal(h.fields.get('#game').height, 700);
});

test('joins with the versioned hello and sends inputs, never positions', () => {
  const h = harness();
  const socket = h.join();
  assert.deepEqual(socket.sent[0], {
    type: 'hello', protocol: 3, game: 'coop-sandbox', ruleset: 'coop-campaign-3', room: 'playtest', patches: true,
  });
  assert.equal(h.fields.get('#connect').blurred, true);
  h.key('keydown', 'KeyD'); h.tick();
  assert.deepEqual(socket.sent.at(-1), { type: 'input', seq: 1, revision: 1, jump_id: 0, input: { axis: 1, jump: false } });
  h.key('keyup', 'KeyD'); h.tick();
  assert.equal(socket.sent.at(-1).input.axis, 0);
});

test('blur and visibility changes immediately release held input', () => {
  const h = harness(); const socket = h.join();
  h.key('keydown', 'KeyD'); h.key('keydown', 'Space'); h.tick();
  h.window.emit('blur');
  assert.deepEqual(socket.sent.at(-1).input, { axis: 0, jump: false });
  h.key('keydown', 'KeyA'); h.tick();
  h.doc.hidden = true; h.doc.emit('visibilitychange');
  assert.deepEqual(socket.sent.at(-1).input, { axis: 0, jump: false });
});

test('multi-touch controls combine and pointer cancellation releases actions', () => {
  const h = harness(); const socket = h.join();
  h.controls[1].emit('pointerdown', { pointerId: 1 });
  h.controls[2].emit('pointerdown', { pointerId: 2 }); h.tick();
  assert.deepEqual(socket.sent.at(-1).input, { axis: 1, jump: true });
  h.controls[1].emit('pointercancel', { pointerId: 1 }); h.tick();
  assert.deepEqual(socket.sent.at(-1).input, { axis: 0, jump: true });
  h.controls[2].emit('lostpointercapture', { pointerId: 2 }); h.tick();
  assert.equal(socket.sent.at(-1).input.jump, false);
});

test('reconnect ignores old socket messages and preserves rejection details', () => {
  const h = harness(); const old = h.join(); const current = h.join();
  old.receive({ type: 'error', message: 'stale connection' });
  assert.ok(!h.fields.get('#status').textContent.includes('stale'));
  current.receive({ type: 'error', message: 'room full' }); current.close();
  assert.equal(h.fields.get('#status').textContent, 'Server: room full');
});

test('backpressure closes instead of retaining old movement indefinitely', () => {
  const h = harness(); const socket = h.join();
  socket.bufferedAmount = 8193; h.tick();
  assert.equal(socket.readyState, 3);
});

test('focused controls support keyboard navigation and held button activation', () => {
  const h = harness(); const socket = h.join();
  h.window.emit('keydown', { code: 'KeyD', target: h.controls[2] });
  h.controls[2].emit('keydown', { code: 'Space' }); h.tick();
  assert.deepEqual(socket.sent.at(-1).input, { axis: 1, jump: true });
  h.controls[2].emit('keyup', { code: 'Space' }); h.tick();
  assert.equal(socket.sent.at(-1).input.jump, false);
  h.controls[2].emit('keydown', { code: 'Enter' }); h.tick();
  assert.equal(socket.sent.at(-1).input.jump, true);
  h.controls[2].emit('blur'); h.tick();
  assert.equal(socket.sent.at(-1).input.jump, false);
});

test('short keyboard and pointer taps capture one ID at press and send neutral release immediately', () => {
  const h = harness(); const socket = h.join();
  h.key('keydown', 'Space');
  assert.equal(socket.sent.at(-1).jump_id, 1);
  assert.equal(socket.sent.at(-1).input.jump, true);
  const count = socket.sent.length;
  h.key('keydown', 'Space'); // OS key repeat is not a new press.
  assert.equal(socket.sent.length, count);
  h.key('keyup', 'Space');
  assert.equal(socket.sent.at(-1).jump_id, 1);
  assert.equal(socket.sent.at(-1).input.jump, false);
  h.controls[2].emit('pointerdown', { pointerId: 7 });
  assert.equal(socket.sent.at(-1).jump_id, 2);
  h.controls[2].emit('pointerup', { pointerId: 7 });
  assert.equal(socket.sent.at(-1).input.jump, false);
  assert.equal(socket.sent.at(-1).jump_id, 2);
});

test('same endpoint and room reconnect uses memory token and resets sequence and jump ID', () => {
  const h = harness(); const old = h.join();
  h.key('keydown', 'Space'); h.tick();
  const current = h.join();
  assert.equal(old.closeCode, 4001);
  assert.equal(current.sent[0].resume_token, 'memory-only-token');
  h.tick();
  assert.equal(current.sent.at(-1).seq, 1);
  assert.equal(current.sent.at(-1).jump_id, 0);
  h.fields.get('#room').value = 'different';
  const separate = h.join();
  assert.equal(current.closeCode, 1000);
  assert.equal('resume_token' in separate.sent[0], false);
});

test('invalid resume token is visible and cleared only for next explicit reconnect', () => {
  const h = harness(); h.join(); const resumed = h.join();
  resumed.receive({ type: 'error', code: 'invalid_resume_token', message: 'invalid resume token' });
  resumed.close();
  assert.equal(h.fields.get('#status').textContent, 'Server: invalid resume token');
  const fresh = h.join();
  assert.equal('resume_token' in fresh.sent[0], false);
});

test('blur cancels a short unsent tap without transmitting its jump ID', () => {
  const h = harness(); const socket = h.join();
  // Delay sends without changing the current connection generation.
  socket.readyState = 0;
  h.key('keydown', 'Space'); h.key('keyup', 'Space');
  const before = socket.sent.length;
  socket.readyState = 1;
  h.window.emit('blur');
  assert.equal(socket.sent.length, before + 1);
  assert.equal(socket.sent.at(-1).jump_id, 0);
  assert.deepEqual(socket.sent.at(-1).input, { axis: 0, jump: false });
  h.tick();
  assert.equal(socket.sent.at(-1).jump_id, 0);
  h.key('keydown', 'Space');
  assert.equal(socket.sent.at(-1).jump_id, 1);
});

test('clearing unsent edges never regresses an already transmitted jump ID', () => {
  const h = harness(); const socket = h.join();
  h.key('keydown', 'Space'); h.key('keyup', 'Space');
  assert.equal(socket.sent.at(-1).jump_id, 1);
  socket.readyState = 0;
  h.key('keydown', 'Space'); h.key('keyup', 'Space');
  socket.readyState = 1;
  h.doc.hidden = true; h.doc.emit('visibilitychange');
  assert.equal(socket.sent.at(-1).jump_id, 1);
  assert.equal(socket.sent.at(-1).input.jump, false);
  h.doc.hidden = false; h.doc.emit('visibilitychange');
  h.key('keydown', 'Space');
  assert.equal(socket.sent.at(-1).jump_id, 2);
});

test('revision changes clear held input and unsent edges without resetting committed IDs or sequence', () => {
  const h = harness(); const socket = h.join();
  h.key('keydown', 'Space'); h.key('keyup', 'Space');
  const seq = socket.sent.at(-1).seq;
  socket.readyState = 0;
  h.controls[1].emit('pointerdown', { pointerId: 1 });
  h.key('keydown', 'Space');
  socket.readyState = 1;
  socket.receive(snapshot({ revision: 2, level: level(2) }));
  assert.deepEqual(socket.sent.at(-1), { type: 'input', seq: seq + 1, revision: 2, jump_id: 1, input: { axis: 0, jump: false } });
  const baselineSeq = socket.sent.at(-1).seq;
  const sentCount = socket.sent.length;
  assert.equal(h.fields.get('#restart').disabled, true);
  assert.equal(h.controls[2].disabled, true);
  h.key('keydown', 'Space');
  h.controls[1].emit('pointerdown', { pointerId: 2 });
  h.advance(100);
  assert.equal(socket.sent.length, sentCount + 1);
  assert.deepEqual(socket.sent.at(-1).input, { axis: 0, jump: false });
  assert.equal(socket.sent.at(-1).jump_id, 1);
  socket.receive(snapshot({ revision: 2, level: level(2), acks: [{ player: 2, seq: baselineSeq }, { player: 1, seq: baselineSeq - 1 }] }));
  assert.equal(h.fields.get('#restart').disabled, true);
  socket.receive(snapshot({ revision: 2, level: level(2), acks: [{ player: 1, seq: baselineSeq }] }));
  assert.equal(h.fields.get('#restart').disabled, false);
  assert.equal(h.controls[2].disabled, false);
  h.tick();
  assert.deepEqual(socket.sent.at(-1).input, { axis: 0, jump: false });
  h.key('keydown', 'Space');
  assert.equal(socket.sent.at(-1).jump_id, 2);
  assert.match(h.fields.get('#status').textContent, /Level 2\/32: Stage 2.*Work together/);
});

test('old revisions and ticks are diagnosed without overwriting level, world, or controller', () => {
  const h = harness(); const socket = h.join();
  const current = snapshot({ revision: 3, level: level(3) });
  socket.receive(current);
  current.acks = [{ player: 1, seq: socket.sent.at(-1).seq }];
  socket.receive(current);
  const before = h.draw();
  socket.receive(snapshot({ revision: 2, level: level(2), controller: 2, world: { ...current.world, tick: 999, key_collected: true } }));
  assert.match(h.fields.get('#status').textContent, /Ignored stale snapshot/);
  assert.deepEqual(h.draw(), before);
  socket.receive(snapshot({ revision: 3, controller: 2, world: { ...current.world, tick: 9, key_collected: true } }));
  assert.deepEqual(h.draw(), before);
  assert.equal(h.fields.get('#restart').disabled, false);
  h.tick(); assert.equal(socket.sent.at(-1).revision, 3);
  socket.receive(snapshot({ revision: 3, level: level(3), world: { ...current.world, tick: 11 } }));
  assert.match(h.fields.get('#status').textContent, /Level 3\/32.*ack pending/);
});

test('higher revision at the same tick starts a baseline; even higher revision with an older tick is rejected', () => {
  const h = harness(); const socket = h.join();
  socket.receive(snapshot());
  socket.receive(snapshot({ revision: 2, level: level(2) }));
  assert.equal(socket.sent.at(-1).revision, 2);
  assert.match(h.fields.get('#status').textContent, /Level 2\/32/);
  const stale = snapshot({ revision: 3, level: level(3) });
  stale.world.tick = 9;
  socket.receive(stale);
  assert.match(h.fields.get('#status').textContent, /Ignored stale snapshot/);
  socket.receive(snapshot({ revision: 2, level: level(2), acks: [{ player: 1, seq: socket.sent.at(-1).seq }] }));
  h.tick(); assert.equal(socket.sent.at(-1).revision, 2);
});

test('controller commands share input sequence and revision; nonowners cannot send commands', () => {
  const h = harness();
  assert.equal(h.fields.get('#restart').disabled, true);
  const socket = h.join();
  const restart = h.fields.get('#restart'), next = h.fields.get('#next');
  assert.equal(restart.disabled, false); assert.equal(next.disabled, true);
  next.emit('click'); assert.equal(socket.sent.length, 1);
  restart.emit('click');
  assert.deepEqual(socket.sent.at(-1), { type: 'restart', seq: 1, revision: 1 });
  h.tick(); assert.equal(socket.sent.at(-1).seq, 2);
  const complete = snapshot(); complete.world.completed = true;
  socket.receive(complete);
  assert.equal(next.disabled, true); assert.equal(next.hidden, true);
  next.emit('click');
  assert.deepEqual(socket.sent.at(-1), { type: 'input', seq: 2, revision: 1, jump_id: 0, input: { axis: 0, jump: false } });
  assert.match(h.fields.get('#status').textContent, /Next stage shortly/);
  socket.receive({ ...complete, controller: 2 });
  assert.equal(restart.disabled, true); assert.equal(next.disabled, true);
  const count = socket.sent.length;
  restart.emit('click'); next.emit('click'); assert.equal(socket.sent.length, count);
  socket.receive({ ...complete, controller: null });
  assert.equal(restart.disabled, true);
  socket.receive({ ...complete, level: level(32), campaign_complete: true });
  assert.equal(next.disabled, true); assert.equal(restart.disabled, false);
  assert.match(h.fields.get('#status').textContent, /Level 32\/32.*Campaign complete! All 32 levels cleared/);
  socket.close(); assert.equal(restart.disabled, true);
});

test('all mechanic glyphs use authoritative object positions, activation and timers', () => {
  const h = harness(); const socket = h.join();
  const kinds = ['plate', 'pressure_gate', 'timed_gate', 'one_way', 'moving_platform', 'conveyor', 'ice', 'spring', 'hazard', 'checkpoint', 'crumble', 'low_gravity', 'wind'];
  const state = snapshot({ level: level(4, kinds.map((kind, id) => ({ id, kind, rect, travel: 0, period: 60 }))) });
  state.world.objects = kinds.map((_, id) => ({ id, rect: { ...rect, x: id * 1024 }, active: id % 2 === 0, timer: id }));
  state.world.checkpoint = rect;
  socket.receive(state);
  const drawn = h.draw();
  for (const [id, object] of state.world.objects.entries()) {
    const label = drawn.find(call => call[0] === 'text' && call[1].includes(`#${id} `));
    assert.ok(label);
    assert.ok(!label[1].startsWith('?'));
    assert.ok(label[1].endsWith(`${object.active ? 'ON' : 'off'} t:${id}`));
    assert.equal(label[2], id * 50);
  }
  assert.ok(drawn.some(call => call[1] === '✓ checkpoint'));
});

test('neutral baseline keeps heartbeats and can acknowledge a later applied frame', () => {
  const h = harness(); const socket = h.join();
  socket.receive(snapshot({ revision: 2, level: level(2) }));
  const first = socket.sent.at(-1).seq;
  h.tick(); h.advance(100);
  const heartbeat = socket.sent.at(-1);
  assert.ok(heartbeat.seq > first);
  assert.deepEqual(heartbeat.input, { axis: 0, jump: false });
  assert.equal(h.controls[0].disabled, true);
  const applied = snapshot({ revision: 2, level: level(2), acks: [{ player: 1, seq: heartbeat.seq }] });
  applied.world.tick = 11;
  socket.receive(applied);
  assert.equal(h.controls[0].disabled, false);
});

test('fresh snapshots cannot hide an unacknowledged baseline forever', () => {
  const h = harness(); const socket = h.join();
  socket.receive(snapshot({ revision: 2, level: level(2) }));
  for (let i = 1; i <= 30; i++) {
    const fresh = snapshot({ revision: 2, level: level(2) }); fresh.world.tick = 10 + i;
    socket.receive(fresh);
    h.advance(100);
  }
  assert.equal(socket.closeCode, 4001);
  assert.equal(h.controls[0].disabled, true);
  h.advance(500);
  assert.equal(h.sockets.length, 2);
  h.sockets[1].open();
  assert.equal(h.sockets[1].sent[0].resume_token, 'memory-only-token');
});

test('stale authority releases held movement and recovers without a manual reconnect', () => {
  const h = harness(); const socket = h.join();
  h.key('keydown', 'KeyD'); h.tick();
  h.advance(751);
  assert.deepEqual(socket.sent.at(-1).input, { axis: 0, jump: false });
  assert.equal(h.controls[1].disabled, true);
  h.advance(4250);
  assert.equal(socket.closeCode, 4001);
  h.advance(500);
  assert.equal(h.sockets.length, 2);
});

test('duplicate world ticks do not renew snapshot freshness', () => {
  const h = harness(); const socket = h.join();
  const same = snapshot(); socket.receive(same);
  for (let i = 0; i < 10; i++) { h.advance(500); socket.receive(same); }
  assert.equal(socket.closeCode, 4001);
});

test('a handshake stuck before open is also bounded', () => {
  const h = harness(); h.fields.get('#join').emit('submit');
  h.advance(5001);
  assert.equal(h.sockets[0].closeCode, 4001);
});

test('ack progress must cover an offered input even after initial synchronization', () => {
  const h = harness(); const socket = h.join();
  h.tick();
  const first = socket.sent.at(-1).seq;
  const acknowledged = snapshot({ acks: [{ player: 1, seq: first }] });
  socket.receive(acknowledged);
  h.advance(100);
  for (let i = 1; i <= 30; i++) {
    const staleAck = snapshot({ acks: [{ player: 1, seq: first }] }); staleAck.world.tick = 10 + i;
    socket.receive(staleAck); h.advance(100);
  }
  assert.equal(socket.closeCode, 4001);
});

test('unchanged inputs use 10 Hz heartbeats while changed inputs are immediate', () => {
  const h = harness(); const socket = h.join();
  h.tick();
  for (let i = 1; i <= 60; i++) {
    const fresh = snapshot({ acks: [{ player: 1, seq: socket.sent.at(-1).seq }] });
    fresh.world.tick = 10 + i; socket.receive(fresh);
    h.advance(1000 / 60);
  }
  const inputs = socket.sent.filter(message => message.type === 'input');
  assert.ok(inputs.length >= 9 && inputs.length <= 11);
  h.key('keydown', 'KeyD'); h.tick();
  assert.equal(socket.sent.at(-1).input.axis, 1);
  h.key('keyup', 'KeyD'); h.tick();
  assert.equal(socket.sent.at(-1).input.axis, 0);
});

test('all thirty-two solution recordings show a death-free correctly staffed finish', () => {
  for (let number = 1; number <= 32; number++) {
    const recording = JSON.parse(readFileSync(new URL(`./solutions/${number}.json`, import.meta.url), 'utf8'));
    assert.equal(recording.level.number, number);
    assert.equal(recording.frames[0].tick, 0);
    assert.ok(recording.frames.length <= 2000);
    assert.match(recording.provenance, /not a proven optimal/);
    const final = recording.frames.at(-1);
    assert.equal(final.completed, true); assert.equal(final.deaths, 0);
    assert.equal(final.players.length, recording.level.required_players);
    assert.ok(recording.frames.every((frame, index) => index === 0 || frame.tick > recording.frames[index - 1].tick));
  }
});

test('solution review leaves live room, disables controls, scrubs and can rejoin', async () => {
  const h = harness(); const socket = h.join();
  await h.review(1);
  assert.equal(socket.closeCode, 1000);
  assert.match(h.fields.get('#status').textContent, /SOLUTION REVIEW/);
  assert.ok(h.controls.every(button => button.disabled));
  const before = socket.sent.length;
  h.key('keydown', 'KeyD'); h.advance(1000); h.draw();
  assert.equal(socket.sent.length, before);
  h.fields.get('#reviewSeek').emit('input', { target: { value: h.fields.get('#reviewSeek').max } });
  h.draw();
  assert.equal(h.fields.get('#reviewSeek').value, h.fields.get('#reviewSeek').max);
  h.join(); assert.equal(h.sockets.length, 2);
  assert.equal(h.sockets[1].sent[0].resume_token, undefined);
});

test('late review fetch cannot overwrite a newly joined live session', async () => {
  let resolve;
  const h = harness({ fetch: () => new Promise(done => { resolve = done; }) });
  const pending = h.review(1);
  h.join();
  resolve({ ok: true, json: async () => JSON.parse(readFileSync(new URL('./solutions/1.json', import.meta.url), 'utf8')) });
  await pending;
  assert.match(h.fields.get('#status').textContent, /You are P1/);
});

test('solution review opens the newest stages', async () => {
  const h = harness(); const socket = h.join();
  await h.review(29);
  assert.equal(socket.closeCode, 1000);
  assert.match(h.fields.get('#status').textContent, /SOLUTION REVIEW · 29\/32/);
  assert.ok(h.controls.every(button => button.disabled));
  const recording29 = JSON.parse(readFileSync(new URL('./solutions/29.json', import.meta.url), 'utf8'));
  assert.equal(h.fields.get('#reviewSeek').max, String(recording29.frames.length - 1));
  await h.review(30);
  assert.match(h.fields.get('#status').textContent, /SOLUTION REVIEW · 30\/32/);
  assert.ok(h.controls.every(button => button.disabled));
  const recording30 = JSON.parse(readFileSync(new URL('./solutions/30.json', import.meta.url), 'utf8'));
  assert.equal(h.fields.get('#reviewSeek').max, String(recording30.frames.length - 1));
  const settled = h.fields.get('#status').textContent;
  const settledMax = h.fields.get('#reviewSeek').max;
  await h.review(33);
  assert.equal(h.fields.get('#status').textContent, settled);
  assert.equal(h.fields.get('#reviewSeek').max, settledMax);
});

test('high key is opaque and painted above translucent low-gravity zone', () => {
  const h = harness(); const socket = h.join();
  const zone = { x: 8 * 1024, y: 3 * 1024, w: 10 * 1024, h: 9 * 1024 };
  const key = { x: 13 * 1024, y: 5 * 1024, w: 512, h: 512 };
  const state = snapshot({ level: { ...level(14, [{ id: 1, kind: 'low_gravity', rect: zone }]), key } });
  state.world.objects = [{ id: 1, rect: zone, active: true, timer: 0 }];
  socket.receive(state);
  const calls = h.draw();
  const zoneIndex = calls.findIndex(call => call[0] === 'rect' && call[1] === 400 && call[2] === 150);
  const keyIndex = calls.findIndex(call => call[0] === 'rect' && call[5] === '#ffe175');
  assert.ok(keyIndex > zoneIndex);
  assert.equal(calls[zoneIndex][6], 0.16); assert.equal(calls[keyIndex][6], 1);
});

test('spikes use triangular silhouettes and door arrivals fade then return on respawn', () => {
  const h = harness(); const socket = h.join();
  const exit = { x: 2000, y: 1000, w: 2048, h: 2048 };
  const hazard = { x: 1000, y: 1000, w: 300, h: 300 };
  const state = snapshot({ level: { ...level(4, [{ id: 1, kind: 'hazard', rect: hazard }]), exit } });
  state.world.key_collected = true;
  state.world.players = [{ id: 1, x: 2000, y: 1000 }];
  state.world.objects = [{ id: 1, rect: hazard, active: true, timer: 0 }];
  socket.receive(state);
  const first = h.draw();
  assert.ok(first.some(call => call[0] === 'line' && call[1] === 1150 / 1024 * 50 && call[2] === 1000 / 1024 * 50));
  h.advance(400);
  assert.ok(h.draw().some(call => call[0] === 'rect' && call[5] === '#b9f479' && call[6] === 0));
  state.world.tick = 20; state.world.deaths = 1; state.world.players[0].x = 0;
  socket.receive(state);
  const respawn = h.draw();
  assert.ok(respawn.some(call => call[0] === 'rect' && call[5] === '#b9f479' && call[6] === 1));
});

test('completed campaign uses slow neutral keepalives and restart wakes immediately', () => {
  const h = harness(); const socket = h.join();
  const final = snapshot({ level: level(28), campaign_complete: true });
  final.world.completed = final.world.key_collected = true;
  socket.receive(final);
  h.tick();
  let previous = socket.sent.at(-1).seq;
  final.world.tick = 11; final.acks = [{ player: 1, seq: previous }]; socket.receive(final);
  for (let index = 1; index <= 120; index++) {
    h.advance(100);
    const offered = socket.sent.at(-1).seq;
    if (offered !== previous) {
      final.world.tick += 1; final.acks = [{ player: 1, seq: offered }]; socket.receive(final);
      previous = offered;
    }
  }
  assert.ok(socket.sent.filter(message => message.type === 'input').length <= 2);
  assert.equal(socket.closeCode, undefined);
  assert.ok(h.controls.every(button => button.disabled));
  h.fields.get('#restart').emit('click');
  assert.equal(socket.sent.at(-1).type, 'restart');
});

test('browser reconstructs opt-in patches and reconnects on a missing base', () => {
  const h = harness(); const socket = h.join();
  socket.receive(snapshot());
  socket.receive({ type: 'patch', revision: 1, base_tick: 10, changes: { world: { tick: 13 } } });
  assert.match(h.fields.get('#status').textContent, /tick 13/);
  socket.receive({ type: 'patch', revision: 1, base_tick: 10, changes: { world: { tick: 16 } } });
  assert.equal(socket.closeCode, 4001);
});

test('large expedition camera pans horizontally and vertically without growing the canvas', () => {
  const h = harness(); const socket = h.join();
  const state = snapshot({ level: { ...level(24), width: 88 * 1024, height: 44 * 1024, required_players: 4 } });
  state.world.players = [{ id: 1, x: 70 * 1024, y: 38 * 1024 }];
  socket.receive(state);
  const calls = h.draw();
  assert.ok(calls.some(call => call[0] === 'translate' && call[1] < 0 && call[2] < 0));
  assert.equal(h.fields.get('#game').width, 1200); assert.equal(h.fields.get('#game').height, 700);
});

test('quick chat sends a preset and renders the server wording', () => {
  const h = harness(); const socket = h.join();
  const state = snapshot();
  state.world.players = [{ id: 1, x: 1024, y: 2048 }, { id: 2, x: 3072, y: 2048 }];
  socket.receive(state);
  const before = socket.sent.length;
  h.chats[2].emit('click');
  assert.deepEqual(socket.sent.at(-1), { type: 'chat', preset: 2 });
  assert.equal(socket.sent.length, before + 1);
  socket.receive({ type: 'chat', player: 2, preset: 2, text: 'Jump!' });
  assert.match(h.fields.get('#chatlog').textContent, /P2: Jump!/);
  const texts = h.draw().filter(call => call[0] === 'text').map(call => call[1]);
  assert.ok(texts.includes('Jump!'));
});

test('quick chat sends nothing while disconnected', () => {
  const h = harness();
  h.chats[0].emit('click');
  assert.equal(h.sockets.length, 0);
});

test('expired chat bubbles are pruned from the bubble map', () => {
  const h = harness(); const socket = h.join();
  const state = snapshot();
  state.world.players = [{ id: 1, x: 1024, y: 2048 }];
  socket.receive(state);
  socket.receive({ type: 'chat', player: 1, preset: 0, text: 'On my way' });
  const live = h.draw().filter(call => call[0] === 'text').map(call => call[1]);
  assert.ok(live.includes('On my way'));
  h.advance(3501);
  const expired = h.draw().filter(call => call[0] === 'text').map(call => call[1]);
  assert.ok(!expired.includes('On my way'));
  const again = h.draw().filter(call => call[0] === 'text').map(call => call[1]);
  assert.ok(!again.includes('On my way'));
});

test('chat buttons are disabled during the cooldown', () => {
  const h = harness(); const socket = h.join();
  const before = socket.sent.length;
  h.chats[2].emit('click');
  assert.equal(socket.sent.length, before + 1);
  assert.ok(h.chats.every(button => button.disabled));
  h.advance(2000);
  assert.ok(h.chats.every(button => !button.disabled));
});

test('chat history clears when the level changes', () => {
  const h = harness(); const socket = h.join();
  socket.receive(snapshot());
  socket.receive({ type: 'chat', player: 2, preset: 2, text: 'Jump!' });
  assert.match(h.fields.get('#chatlog').textContent, /P2: Jump!/);
  socket.receive(snapshot({ revision: 2, level: level(2) }));
  assert.equal(h.fields.get('#chatlog').textContent, '');
});

test('chat cooldown does not survive a reconnect', () => {
  const h = harness(); h.join();
  h.chats[2].emit('click');
  assert.ok(h.chats.every(button => button.disabled));
  h.join();
  assert.ok(h.chats.every(button => !button.disabled));
});

test('a rescue shows a run-lost notice that expires', () => {
  const h = harness(); const socket = h.join();
  const state = snapshot();
  state.world.players = [{ id: 1, x: 1024, y: 2048 }];
  socket.receive(state);
  assert.ok(!h.draw().some(call => call[0] === 'text' && /Run lost/.test(call[1])));
  const rescued = snapshot();
  rescued.world.players = [{ id: 1, x: 1024, y: 2048 }];
  rescued.world.rescued_at = 1234;
  socket.receive(rescued);
  const shown = h.draw();
  assert.ok(shown.some(call => call[0] === 'text' && /Run lost/.test(call[1])),
    'the rescue notice must be drawn');
  h.advance(2600);
  assert.ok(!h.draw().some(call => call[0] === 'text' && /Run lost/.test(call[1])),
    'the notice must expire');
});

test('a rescue is shown once for one marker', () => {
  const h = harness(); const socket = h.join();
  const state = snapshot();
  state.world.players = [{ id: 1, x: 1024, y: 2048 }];
  state.world.rescued_at = 77;
  socket.receive(state);
  const first = h.draw().filter(call => call[0] === 'text' && /Run lost/.test(call[1])).length;
  socket.receive(state);
  const second = h.draw().filter(call => call[0] === 'text' && /Run lost/.test(call[1])).length;
  assert.equal(first, 1);
  // The banner keeps drawing while live, but does not stack copies.
  assert.ok(second <= 1, 'the notice must not stack per snapshot');
});

test('a rescue marker from an old session does not fire after reconnect', () => {
  const h = harness();
  const first = h.join();
  const state = snapshot();
  state.world.players = [{ id: 1, x: 1024, y: 2048 }];
  state.world.rescued_at = 5;
  first.receive(state);
  assert.ok(h.draw().some(call => call[0] === 'text' && /Run lost/.test(call[1])));
  h.join();
  const after = h.draw();
  assert.ok(!after.some(call => call[0] === 'text' && /Run lost/.test(call[1])),
    'a new session must not inherit the previous rescue banner');
});
