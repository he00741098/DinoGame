// Transport-independent payload counters. Never store message content or tokens.
class NetworkStatistics {
  sentBytes = 0; receivedBytes = 0; sentMessages = 0; receivedMessages = 0;
  pending = new Map(); ackMs; gapMs; jitterMs = 0; previousSnapshot; started;
  begin(now) {
    this.started ??= now;
    this.pending.clear(); this.ackMs = this.gapMs = this.previousSnapshot = undefined; this.jitterMs = 0;
  }
  sent(text, sequence, now) {
    this.sentBytes += new TextEncoder().encode(text).length; ++this.sentMessages;
    if (sequence !== undefined) {
      if (this.pending.size >= 128) this.pending.delete(this.pending.keys().next().value);
      this.pending.set(sequence, now);
    }
  }
  received(text) { this.receivedBytes += new TextEncoder().encode(text).length; ++this.receivedMessages; }
  snapshot(ack, now) {
    if (this.previousSnapshot !== undefined) {
      const gap = Math.max(0, now - this.previousSnapshot);
      if (this.gapMs !== undefined) this.jitterMs += (Math.abs(gap - this.gapMs) - this.jitterMs) / 16;
      this.gapMs = gap;
    }
    this.previousSnapshot = now;
    for (const [sequence, time] of this.pending) if (sequence <= ack) {
      this.ackMs = Math.max(0, now - time); this.pending.delete(sequence);
    }
  }
  summary(now) {
    const seconds = Math.max(1, (now - (this.started ?? now)) / 1000);
    return `Payload ↑${(this.sentBytes / seconds / 1024).toFixed(1)} ↓${(this.receivedBytes / seconds / 1024).toFixed(1)} KiB/s` +
      ` · ↑${(this.sentBytes / 1024).toFixed(0)} ↓${(this.receivedBytes / 1024).toFixed(0)} KiB total` +
      ` · applied ack ${this.ackMs?.toFixed(0) ?? 'pending'} ms · jitter ${this.jitterMs.toFixed(0)} ms`;
  }
}

class SnapshotWireDecoder {
  base;
  decode(message) {
    if (message.type === 'patch') {
      if (!this.base || message.base_tick !== this.base.world.tick || message.revision !== this.base.revision ||
          !message.changes || ['type', 'level', 'revision'].some(key => Object.hasOwn(message.changes, key))) throw new Error('Invalid patch base');
      message = this.merge(this.base, message.changes, 0);
      if (JSON.stringify(message).length > 65536) throw new Error('Oversized snapshot');
    }
    if (message.type === 'snapshot') this.base = message;
    return message;
  }
  merge(base, changes, depth) {
    if (depth >= 16 || !changes || Array.isArray(changes) || typeof changes !== 'object') throw new Error('Invalid patch');
    const result = { ...base };
    for (const [key, value] of Object.entries(changes)) {
      if (['__proto__', 'constructor', 'prototype'].includes(key) || !Object.hasOwn(base, key)) throw new Error('Invalid patch key');
      result[key] = value && typeof value === 'object' && !Array.isArray(value) && base[key] && !Array.isArray(base[key]) && typeof base[key] === 'object'
        ? this.merge(base[key], value, depth + 1) : value;
    }
    return result;
  }
}
