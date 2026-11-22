import Foundation

/// Presentation only: buffered authority plus short, bounded dead reckoning.
/// No inputs, collisions, simulation constants, game flags or authoritative state.
public struct MotionPoint: Sendable, Equatable {
    public var x: Double
    public var y: Double
    public init(x: Double, y: Double) { self.x = x; self.y = y }
}
public struct MotionBody: Sendable {
    public let id: UInt32
    public let position: MotionPoint
    public let velocity: MotionPoint // units/second
    public init(id: UInt32, position: MotionPoint, velocity: MotionPoint) {
        self.id = id; self.position = position; self.velocity = velocity
    }
}
public struct MotionTimeline: Sendable {
    private struct Frame: Sendable { let time: Double; let received: Double; let bodies: [MotionBody] }
    private struct Output: Sendable { let point: MotionPoint; let time: Double }
    private var frames: [Frame] = []
    private var outputs: [UInt32: Output] = [:]
    private let extrapolationCap: Double // seconds; 0 = never predict past the newest sample
    private let smoothingDuration: Double // seconds; 0 = return the sample without easing
    public init(extrapolationCap: Double = 0.1, smoothingDuration: Double = 0.035) {
        self.extrapolationCap = extrapolationCap.isFinite ? max(0, extrapolationCap) : 0
        self.smoothingDuration = smoothingDuration.isFinite ? max(0, smoothingDuration) : 0
    }
    public mutating func reset() { frames.removeAll(); outputs.removeAll() }
    public mutating func append(time: Double, received: Double, bodies: [MotionBody]) {
        guard time.isFinite, received.isFinite, bodies.count <= 4,
              bodies.allSatisfy({ [$0.position.x, $0.position.y, $0.velocity.x, $0.velocity.y].allSatisfy(\.isFinite) }) else { return }
        if let last = frames.last, time < last.time { reset() }
        if frames.last?.time == time { return }
        if let last = frames.last, bodies.contains(where: { body in
            last.bodies.first(where: { $0.id == body.id }).map { hypot(body.position.x - $0.position.x, body.position.y - $0.position.y) > 4096 } ?? false
        }) { reset() }
        frames.append(Frame(time: time, received: received, bodies: bodies))
        if frames.count > 8 { frames.removeFirst() }
        outputs = outputs.filter { id, _ in bodies.contains { $0.id == id } }
    }
    public mutating func position(id: UInt32, now: Double, local: Bool = false) -> MotionPoint? {
        let samples = frames.compactMap { frame in frame.bodies.first(where: { $0.id == id }).map { (frame.time, $0) } }
        guard now.isFinite, let last = samples.last, let offset = frames.map({ $0.received - $0.time }).min() else { return nil }
        let target = min(last.0 + extrapolationCap, now - offset - (local ? 0 : 0.1))
        var point = samples[0].1.position
        if target >= last.0 {
            let dt = max(0, min(extrapolationCap, target - last.0))
            var ay = 0.0
            if samples.count >= 2 {
                let before = samples[samples.count - 2]
                if last.1.velocity.y != 0 && before.1.velocity.y != 0 {
                    ay = max(-200_000, min(200_000, (last.1.velocity.y - before.1.velocity.y) / (last.0 - before.0)))
                }
            }
            var dx = last.1.velocity.x * dt, dy = last.1.velocity.y * dt + ay * dt * dt / 2
            let distance = hypot(dx, dy)
            if distance > 1536 { dx *= 1536 / distance; dy *= 1536 / distance }
            point = MotionPoint(x: last.1.position.x + dx, y: last.1.position.y + dy)
        } else {
            for index in 1..<samples.count where target >= samples[index - 1].0 && target <= samples[index].0 {
                let a = samples[index - 1], b = samples[index]
                let t = (target - a.0) / (b.0 - a.0)
                point = MotionPoint(x: a.1.position.x + (b.1.position.x - a.1.position.x) * t,
                                    y: a.1.position.y + (b.1.position.y - a.1.position.y) * t)
                break
            }
        }
        if smoothingDuration > 0, let previous = outputs[id] {
            let dt = max(0, min(0.1, now - previous.time))
            let alpha = 1 - exp(-dt / smoothingDuration)
            point = MotionPoint(x: previous.point.x + (point.x - previous.point.x) * alpha,
                                y: previous.point.y + (point.y - previous.point.y) * alpha)
        }
        outputs[id] = Output(point: point, time: now)
        return point
    }
}
