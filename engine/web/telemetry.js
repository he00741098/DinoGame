// Local-only playtest evidence: bounded in-memory buffers, exported only when
// the operator asks for a download. Nothing here is ever transmitted.
const SAMPLE_CAP = 4096;
const EVENT_CAP = 512;

function pushBounded(buffer, value, cap) {
  if (buffer.length >= cap) buffer.shift();
  buffer.push(value);
}
function stats(values) {
  if (!values.length) return { median: null, p95: null };
  const ordered = [...values].sort((a, b) => a - b);
  const middle = ordered.length >> 1;
  const median = ordered.length % 2 ? ordered[middle] : (ordered[middle - 1] + ordered[middle]) / 2;
  // Nearest rank, so p95 of one sample is that sample rather than an interpolation.
  return { median, p95: ordered[Math.max(0, Math.ceil(ordered.length * 0.95) - 1)] };
}

class SessionTelemetry {
  gaps = []; ackLatencies = []; frames = []; events = [];
  startedAt; previousSnapshotAt; previousFrameAt;
  constructor(now) { this.startedAt = now; }
  recordSnapshot(now) {
    if (this.previousSnapshotAt !== undefined) pushBounded(this.gaps, Math.max(0, now - this.previousSnapshotAt), SAMPLE_CAP);
    this.previousSnapshotAt = now;
  }
  recordAck(latencyMs) { pushBounded(this.ackLatencies, Math.max(0, latencyMs), SAMPLE_CAP); }
  recordFrame(now) {
    if (this.previousFrameAt !== undefined) {
      const duration = now - this.previousFrameAt;
      // A suspended draw loop (hidden tab) is a pause, not a frame sample.
      if (duration <= 1000) pushBounded(this.frames, duration, SAMPLE_CAP);
    }
    this.previousFrameAt = now;
  }
  recordEvent(type, now, detail) {
    pushBounded(this.events, { type, at: now - this.startedAt, ...detail }, EVENT_CAP);
  }
  summary(now) {
    return {
      started_at: this.startedAt,
      ended_at: now,
      counts: { snapshots: this.gaps.length, acks: this.ackLatencies.length, frames: this.frames.length, events: this.events.length },
      snapshot_gap: stats(this.gaps),
      ack_latency: stats(this.ackLatencies),
      frame_time: stats(this.frames),
      events: [...this.events],
    };
  }
}

export { SessionTelemetry, SAMPLE_CAP, EVENT_CAP };
// The page consumes shared classes from the global scope (NetworkStatistics,
// MotionTimeline), so the module publishes itself for client.js.
globalThis.SessionTelemetry = SessionTelemetry;
