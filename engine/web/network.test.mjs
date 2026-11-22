import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';

test('network counters count UTF-8 bytes and applied acks, not duplicate ack age', () => {
  const context = vm.createContext({ TextEncoder });
  vm.runInContext(readFileSync(new URL('./network.js', import.meta.url), 'utf8'), context);
  const stats = vm.runInContext('new NetworkStatistics()', context);
  stats.begin(0); stats.sent('é', 1, 0); stats.received('hello');
  stats.snapshot(1, 50);
  assert.equal(stats.sentBytes, 2); assert.equal(stats.receivedBytes, 5);
  assert.equal(stats.ackMs, 50);
  stats.snapshot(1, 100); assert.equal(stats.ackMs, 50);
  for (let seq = 2; seq <= 1000; seq++) stats.sent('x', seq, 100);
  assert.equal(stats.pending.size, 128);
  stats.begin(1000); assert.equal(stats.pending.size, 0); assert.equal(stats.ackMs, undefined);
  assert.equal(stats.receivedBytes, 5);
});

test('patches reconstruct exact bases, replace arrays, retain null and reject stale bases', () => {
  const context = vm.createContext({ TextEncoder });
  vm.runInContext(readFileSync(new URL('./network.js', import.meta.url), 'utf8'), context);
  const wire = vm.runInContext('new SnapshotWireDecoder()', context);
  const full = { type: 'snapshot', revision: 1, level: { number: 1 }, world: { tick: 1, checkpoint: { x: 1 } }, acks: [] };
  wire.decode(full);
  const patch = { type: 'patch', revision: 1, base_tick: 1, changes: { world: { tick: 7, checkpoint: null }, acks: [{ player: 1, seq: 3 }] } };
  const next = wire.decode(patch);
  assert.equal(next.world.tick, 7); assert.equal(next.world.checkpoint, null);
  assert.equal(full.world.tick, 1);
  assert.throws(() => wire.decode(patch));
  assert.throws(() => wire.decode({ ...patch, base_tick: 7, revision: 2 }));
  assert.throws(() => wire.decode({ ...patch, base_tick: 7, changes: JSON.parse('{"__proto__":{"polluted":true}}') }));
});

test('merged patches are rejected on UTF-8 byte length, not UTF-16 length', () => {
  const context = vm.createContext({ TextEncoder });
  vm.runInContext(readFileSync(new URL('./network.js', import.meta.url), 'utf8'), context);
  const wire = vm.runInContext('new SnapshotWireDecoder()', context);
  const full = { type: 'snapshot', revision: 1, level: { number: 1 }, world: { tick: 1, checkpoint: { x: 1 } }, acks: [] };
  wire.decode(full);
  // 32000 astral emojis: 64000 UTF-16 code units but 128000 UTF-8 bytes, so the
  // merged snapshot stays under the limit only if code units are counted.
  const big = '\u{1F600}'.repeat(32000);
  const merged = { ...full, world: { ...full.world, tick: big } };
  const text = JSON.stringify(merged);
  assert.ok(text.length <= 65536, `UTF-16 length ${text.length} is within the limit`);
  assert.ok(new TextEncoder().encode(text).length > 65536, 'UTF-8 byte length exceeds the limit');
  assert.throws(
    () => wire.decode({ type: 'patch', revision: 1, base_tick: 1, changes: { world: { tick: big } } }),
    /Oversized snapshot/,
  );
});
