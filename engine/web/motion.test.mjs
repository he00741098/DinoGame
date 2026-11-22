import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';

function timeline(options = {}) {
  const context = vm.createContext({ Math, Number });
  vm.runInContext(readFileSync(new URL('./motion.js', import.meta.url), 'utf8'), context);
  return vm.runInContext(`new MotionTimeline(${JSON.stringify(options)})`, context);
}
const body = (x, y = 0, vx = 0, vy = 0) => [{ id: 1, x, y, vx, vy }];

test('buffered motion interpolates between authority frames instead of waiting for packets', () => {
  const m = timeline();
  m.append(0, 10, body(0)); m.append(0.05, 10.05, body(1000));
  const p = m.position(1, 10.125, true); // local: no extra 100 ms buffer
  assert.ok(p.x > 0 && p.x <= 1000, `x=${p.x}`);
});

test('missing packets extrapolate only briefly and within a hard distance cap', () => {
  const m = timeline();
  m.append(0, 10, body(0, 0, 9600, 0)); m.append(0.05, 10.05, body(480, 0, 9600, 0));
  let last = 0;
  for (let t = 10.05; t < 11; t += 0.016) last = m.position(1, t, true).x;
  assert.ok(last <= 480 + 1536 + 1, `frozen-then-capped x=${last}`);
  assert.ok(last > 480, 'moves on through a short stall');
});

test('teleports, rewinds and disappearing players reset presentation history', () => {
  const m = timeline();
  m.append(1, 1, body(0)); m.append(1.05, 1.05, body(100000));
  assert.equal(m.frames.length, 1);
  m.append(0.5, 1.1, body(10)); // authority tick went backward
  assert.equal(m.frames.length, 1);
  m.append(0.55, 1.15, []);
  assert.equal(m.outputs.size, 0);
});

test('non-finite or oversized input is ignored and never produces NaN positions', () => {
  const m = timeline();
  m.append(NaN, 1, body(0)); m.append(1, 1, [{ id: 1, x: Infinity, y: 0, vx: 0, vy: 0 }]);
  assert.equal(m.position(1, 2), undefined);
});

test('zero extrapolation never invents positions', () => {
  const m = timeline({ extrapolationCapMs: 0 });
  m.append(0, 10, body(0, 0, 9600, 0)); m.append(0.05, 10.05, body(480, 0, 9600, 0));
  for (const now of [10.1, 10.5, 11, 20]) {
    const p = m.position(1, now, true);
    assert.equal(p.x, 480, `frozen at newest authority sample, now=${now} x=${p.x}`);
    assert.equal(p.y, 0, `no invented vertical drift, now=${now} y=${p.y}`);
  }
});

test('zero smoothing passes interpolated positions through', () => {
  const m = timeline({ smoothingMs: 0 });
  m.append(0, 1, body(0)); m.append(0.0625, 1.0625, body(1000)); // exact binary times
  assert.equal(m.position(1, 1.03125, true).x, 500); // t = 0.5 between 0 and 1000
  assert.equal(m.position(1, 1.046875, true).x, 750); // t = 0.75, no exponential lag behind 750
});

test('defaults still extrapolate', () => {
  const m = timeline();
  m.append(0, 10, body(0, 0, 9600, 0)); m.append(0.05, 10.05, body(480, 0, 9600, 0));
  const p = m.position(1, 10.2, true);
  assert.ok(p.x > 480, `default moves on past the newest sample x=${p.x}`);
  assert.ok(p.x <= 480 + 1536, `still distance-capped x=${p.x}`);
});
