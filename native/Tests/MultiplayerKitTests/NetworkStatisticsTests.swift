import Testing
@testable import MultiplayerKit

@Test func unchangedInputUsesLeaseHeartbeatButChangesAndJumpEdgesAreImmediate() {
    var policy = InputSendPolicy()
    let first = policy.shouldSend(GameInput(), jumpID: 0, revision: 1, at: 0)
    #expect(first)
    for tick in 1..<6 {
        let duplicate = policy.shouldSend(GameInput(), jumpID: 0, revision: 1, at: Double(tick) / 60)
        #expect(!duplicate)
    }
    let heartbeat = policy.shouldSend(GameInput(), jumpID: 0, revision: 1, at: 0.101)
    let edge = policy.shouldSend(GameInput(), jumpID: 1, revision: 1, at: 0.102)
    let revision = policy.shouldSend(GameInput(), jumpID: 1, revision: 2, at: 0.103)
    let forced = policy.shouldSend(GameInput(), jumpID: 1, revision: 2, at: 0.104, force: true)
    #expect(heartbeat && edge && revision && forced)
}

@Test func payloadAccountingAndAppliedAcknowledgmentAreBoundedAndResetOnReconnect() {
    var stats = NetworkStatistics()
    stats.beginConnection(at: 0)
    stats.sent(bytes: 100, sequence: 1, at: 0)
    stats.received(bytes: 500)
    stats.snapshot(ack: 1, at: 0.05)
    #expect(stats.sentBytes == 100 && stats.receivedBytes == 500)
    #expect(stats.ackMilliseconds == 50)
    stats.snapshot(ack: 1, at: 0.1)
    #expect(stats.ackMilliseconds == 50) // Duplicate acks do not grow latency.
    stats.beginConnection(at: 1)
    #expect(stats.ackMilliseconds == nil)
    #expect(stats.receivedBytes == 500)
    for i in 1...1000 { stats.sent(bytes: 1, sequence: UInt32(i), at: 1) }
    stats.snapshot(ack: 1000, at: 1.1)
    #expect(stats.ackMilliseconds! < 101)
}
