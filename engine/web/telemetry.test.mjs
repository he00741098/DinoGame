import test from 'node:test';
import assert from 'node:assert/strict';
import { SessionTelemetry, SAMPLE_CAP, EVENT_CAP } from './telemetry.js';

test('snapshot gaps produce median and p95 and the sample buffer stays bounded', () => {
  const telemetry = new SessionTelemetry(0);
  let now = 0;
  telemetry.recordSnapshot(now); // First arrival only starts the clock.
  for (const gap of [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]) {
    now += gap;
    telemetry.recordSnapshot(now);
  }
  const summary = telemetry.summary(now);
  assert.equal(summary.counts.snapshots, 10);
  assert.equal(summary.snapshot_gap.median, 55);
  assert.equal(summary.snapshot_gap.p95, 100);
  for (let index = 0; index < SAMPLE_CAP + 500; index++) {
    now += 5;
    telemetry.recordSnapshot(now);
  }
  const bounded = telemetry.summary(now);
  assert.equal(telemetry.gaps.length, SAMPLE_CAP);
  assert.equal(bounded.counts.snapshots, SAMPLE_CAP);
  // Drop-oldest keeps only the recent window: every retained gap is 5 ms.
  assert.equal(bounded.snapshot_gap.median, 5);
  assert.equal(bounded.snapshot_gap.p95, 5);
});

test('events carry relative timestamps and survive a JSON round trip', () => {
  const telemetry = new SessionTelemetry(1000);
  telemetry.recordEvent('join', 1250);
  telemetry.recordEvent('level_change', 3000, { level: 2 });
  telemetry.recordEvent('death', 3400, { deaths: 1 });
  telemetry.recordEvent('rescue', 3500, { rescued_at: 120 });
  const summary = telemetry.summary(4000);
  const restored = JSON.parse(JSON.stringify(summary));
  assert.deepEqual(restored, summary);
  assert.deepEqual(restored.events, [
    { type: 'join', at: 250 },
    { type: 'level_change', at: 2000, level: 2 },
    { type: 'death', at: 2400, deaths: 1 },
    { type: 'rescue', at: 2500, rescued_at: 120 },
  ]);
  assert.equal(restored.started_at, 1000);
  assert.equal(restored.ended_at, 4000);
  for (let index = 0; index < EVENT_CAP + 100; index++) telemetry.recordEvent('resume', 4000 + index);
  assert.equal(telemetry.events.length, EVENT_CAP);
  assert.equal(telemetry.summary(5000).counts.events, EVENT_CAP);
});

test('an empty session summarises to zeros and nulls instead of NaN', () => {
  const summary = new SessionTelemetry(0).summary(5000);
  assert.equal(summary.started_at, 0);
  assert.equal(summary.ended_at, 5000);
  assert.deepEqual(summary.counts, { snapshots: 0, acks: 0, frames: 0, events: 0 });
  assert.deepEqual(summary.snapshot_gap, { median: null, p95: null });
  assert.deepEqual(summary.ack_latency, { median: null, p95: null });
  assert.deepEqual(summary.frame_time, { median: null, p95: null });
  assert.deepEqual(summary.events, []);
  assert.deepEqual(JSON.parse(JSON.stringify(summary)), summary);
  assert.ok([summary.started_at, summary.ended_at].every(Number.isFinite));
});
