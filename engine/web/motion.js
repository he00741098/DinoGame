// Original presentation math only; never feeds a guessed position to authority.
class MotionTimeline {
  frames = []; outputs = new Map();
  constructor({ extrapolationCapMs = 100, smoothingMs = 35 } = {}) {
    this.extrapolationCapMs = extrapolationCapMs;
    this.smoothingMs = smoothingMs;
  }
  reset() { this.frames = []; this.outputs.clear(); }
  append(time, received, bodies) {
    if (![time, received].every(Number.isFinite) || bodies.length > 4 ||
        !bodies.every(p => [p.x, p.y, p.vx, p.vy].every(Number.isFinite))) return;
    if (this.frames.length && time < this.frames.at(-1).time) this.reset();
    if (this.frames.at(-1)?.time === time) return;
    const last = this.frames.at(-1);
    if (last && bodies.some(p => { const before = last.bodies.find(b => b.id === p.id); return before && Math.hypot(p.x - before.x, p.y - before.y) > 4096; })) this.reset();
    this.frames.push({ time, received, bodies: bodies.map(p => ({ ...p })) });
    if (this.frames.length > 8) this.frames.shift();
    for (const id of this.outputs.keys()) if (!bodies.some(p => p.id === id)) this.outputs.delete(id);
  }
  position(id, now, local = false) {
    const samples = this.frames.flatMap(frame => { const body = frame.bodies.find(p => p.id === id); return body ? [{ time: frame.time, body }] : []; });
    const last = samples.at(-1);
    if (!last || !Number.isFinite(now)) return;
    const offset = Math.min(...this.frames.map(frame => frame.received - frame.time));
    const cap = this.extrapolationCapMs / 1000;
    const target = Math.min(last.time + cap, now - offset - (local ? 0 : 0.1));
    let point = { x: samples[0].body.x, y: samples[0].body.y };
    if (target >= last.time) {
      const dt = Math.max(0, Math.min(cap, target - last.time));
      const before = samples.at(-2);
      const ay = before && last.body.vy !== 0 && before.body.vy !== 0
        ? Math.max(-200000, Math.min(200000, (last.body.vy - before.body.vy) / (last.time - before.time))) : 0;
      let dx = last.body.vx * dt, dy = last.body.vy * dt + ay * dt * dt / 2;
      const distance = Math.hypot(dx, dy);
      if (distance > 1536) { dx *= 1536 / distance; dy *= 1536 / distance; }
      point = { x: last.body.x + dx, y: last.body.y + dy };
    } else {
      for (let index = 1; index < samples.length; index++) {
        const a = samples[index - 1], b = samples[index];
        if (target < a.time || target > b.time) continue;
        const t = (target - a.time) / (b.time - a.time);
        point = { x: a.body.x + (b.body.x - a.body.x) * t, y: a.body.y + (b.body.y - a.body.y) * t }; break;
      }
    }
    const previous = this.outputs.get(id);
    if (previous && this.smoothingMs > 0) {
      const alpha = 1 - Math.exp(-Math.max(0, Math.min(0.1, now - previous.time)) / (this.smoothingMs / 1000));
      point = { x: previous.point.x + (point.x - previous.point.x) * alpha, y: previous.point.y + (point.y - previous.point.y) * alpha };
    }
    this.outputs.set(id, { point, time: now }); return point;
  }
}
