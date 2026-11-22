import Foundation

/// Application payload accounting only: excludes TLS/TCP/WebSocket/IP framing,
/// retransmissions and other applications. No addresses, credentials or content.
public struct NetworkStatistics: Sendable {
    public private(set) var sentBytes = 0
    public private(set) var receivedBytes = 0
    public private(set) var sentMessages = 0
    public private(set) var receivedMessages = 0
    public private(set) var ackMilliseconds: Double?
    public private(set) var snapshotGapMilliseconds: Double?
    public private(set) var jitterMilliseconds = 0.0
    private var pending: [(UInt32, Double)] = []
    private var previousSnapshot: Double?
    private var previousGap: Double?
    private var started: Double?

    public init() {}
    public mutating func beginConnection(at time: Double) {
        if started == nil { started = time }
        pending.removeAll(); previousSnapshot = nil; previousGap = nil
        ackMilliseconds = nil; snapshotGapMilliseconds = nil; jitterMilliseconds = 0
    }
    public mutating func sent(bytes: Int, sequence: UInt32? = nil, at time: Double) {
        sentBytes += max(0, bytes); sentMessages += 1
        if let sequence {
            if pending.count == 128 { pending.removeFirst() }
            pending.append((sequence, time))
        }
    }
    public mutating func received(bytes: Int) {
        receivedBytes += max(0, bytes); receivedMessages += 1
    }
    public mutating func snapshot(ack: UInt32?, at time: Double) {
        if let previousSnapshot {
            let gap = max(0, time - previousSnapshot) * 1000
            snapshotGapMilliseconds = gap
            if let previousGap { jitterMilliseconds += (abs(gap - previousGap) - jitterMilliseconds) / 16 }
            previousGap = gap
        }
        previousSnapshot = time
        if let ack, let entry = pending.last(where: { $0.0 <= ack }) {
            ackMilliseconds = max(0, time - entry.1) * 1000
            pending.removeAll { $0.0 <= ack }
        }
    }
    public func summary(at time: Double) -> String {
        let seconds = max(1, time - (started ?? time))
        let latency = ackMilliseconds.map { String(format: "%.0f ms", $0) } ?? "pending"
        return String(format: "Payload ↑%.1f ↓%.1f KiB/s · ↑%.0f ↓%.0f KiB total · applied ack %@ · jitter %.0f ms",
                      Double(sentBytes) / seconds / 1024, Double(receivedBytes) / seconds / 1024,
                      Double(sentBytes) / 1024, Double(receivedBytes) / 1024, latency, jitterMilliseconds)
    }
}

/// Poll controls at display cadence, but only transmit changes or a 100 ms lease
/// heartbeat. Discrete jump IDs and revision changes are never coalesced away.
public struct InputSendPolicy: Sendable {
    private var previous: GameInput?
    private var jumpID: UInt32 = 0
    private var revision: UInt32 = 0
    private var lastSend = -Double.infinity
    public init() {}
    public mutating func shouldSend(_ input: GameInput, jumpID: UInt32, revision: UInt32, at time: Double, force: Bool = false) -> Bool {
        guard force || input != previous || jumpID != self.jumpID || revision != self.revision || time - lastSend >= 0.1 else { return false }
        previous = input; self.jumpID = jumpID; self.revision = revision; lastSend = time
        return true
    }
}
