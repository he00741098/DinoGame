import Foundation
import Testing
@testable import MultiplayerKit

@Test func resumeSecretsAreScopedBoundedAndMemoryOnly() throws {
    let primary = try ConnectionRoute("wss://primary.example", allowLocalInsecure: false)
    let fallback = try ConnectionRoute("wss://fallback.example", allowLocalInsecure: false)
    var store = ResumeStore()
    store.save("secret", for: primary, room: "one")
    #expect(store.token(for: primary, room: "one") == "secret")
    #expect(store.token(for: fallback, room: "one") == nil)
    #expect(store.token(for: primary, room: "two") == nil)
    for index in 0..<100 { store.save("token-\(index)", for: primary, room: "room-\(index)") }
    #expect(store.token(for: primary, room: "one") == nil)
    store.clear()
    #expect(store.token(for: primary, room: "room-99") == nil)
}

@Test func duplicateWelcomeAndUnsentAcksCannotChangeAcceptedState() throws {
    var stream = SnapshotStream()
    guard case .welcome(let welcome) = try ServerEvent.decode(welcomeData()) else { return }
    try stream.start(welcome)
    #expect(throws: ProtocolError.invalidOrder) { try stream.start(welcome) }
    guard case .snapshot(let badAck) = try ServerEvent.decode(snapshotData(tick: 5, ack: 2)) else { return }
    #expect(throws: ProtocolError.invalidSnapshot) { try stream.accept(badAck, lastSentSequence: 1) }
    #expect(stream.lastTick == nil)
}

@Test func hostileGeometryFailsBeforeRenderingOrIntegerArithmetic() throws {
    let source = String(decoding: welcomeData(), as: UTF8.self)
    for changed in [
        source.replacingOccurrences(of: "\"width\":24576", with: "\"width\":0"),
        source.replacingOccurrences(of: "\"w\":24576", with: "\"w\":9223372036854775807"),
        source.replacingOccurrences(of: "\"x\":9216", with: "\"x\":-1"),
        source.replacingOccurrences(of: "\"height\":14336", with: "\"height\":100000000000000000000000"),
    ] {
        #expect(throws: (any Error).self) { try ServerEvent.decode(Data(changed.utf8)) }
    }
}

@Test func malformedAndTruncatedFramesAreRejected() {
    let bytes = Array(welcomeData())
    for count in stride(from: 0, to: bytes.count - 1, by: 11) {
        #expect(throws: (any Error).self) { try ServerEvent.decode(Data(bytes.prefix(count))) }
    }
    for text in ["null", "[]", "{}", "true", "\"snapshot\"", "{\"type\":1}", "{\"type\":\"snapshot\",\"world\":null}"] {
        #expect(throws: (any Error).self) { try ServerEvent.decode(Data(text.utf8)) }
    }
}

@Test func commandsRefuseInvalidMutableInputAndZeroSequences() {
    var input = GameInput()
    input.axis = Int.max
    #expect(throws: ProtocolError.invalidInput) { try ClientCommand.input(seq: 1, input: input, jumpID: 0, revision: 0) }
    #expect(throws: ProtocolError.invalidInput) { try ClientCommand.input(seq: 0, input: GameInput(), jumpID: 0, revision: 0) }
}

@Test func initialOrResumedSnapshotMayOmitUnappliedAcks() throws {
    var stream = SnapshotStream()
    guard case .welcome(let welcome) = try ServerEvent.decode(welcomeData()) else { return }
    try stream.start(welcome)
    let initial = String(decoding: snapshotData(), as: UTF8.self)
        .replacingOccurrences(of: "[{\"player\":1,\"seq\":0}]", with: "[]")
    guard case .snapshot(let snapshot) = try ServerEvent.decode(Data(initial.utf8)) else { return }
    #expect(try stream.accept(snapshot, lastSentSequence: 0))
}

@Test func pauseDiscardsUnsentJumpWithoutRegressingCommittedIDs() {
    var input = InputState()
    input.set(.jump, source: "jump", pressed: true)
    #expect(input.jumpID == 1)
    input.clear()
    #expect(input.jumpID == 0)
    #expect(!input.sample().jump)
    input.set(.jump, source: "jump", pressed: true)
    #expect(input.sample().jump)
    input.markTransmitted()
    input.clear()
    #expect(input.jumpID == 1)
    input.set(.jump, source: "jump", pressed: true)
    #expect(input.jumpID == 2)
    input.clear()
    #expect(input.jumpID == 1)
}

@Test func healthyConnectionResetsBackoffAndKeepsItsSuccessfulRoute() throws {
    var recovery = RecoveryState()
    for _ in 0..<20 { recovery.failed(afterHealthyConnection: false, routeCount: 2) }
    #expect(recovery.failures == 4)
    let workingRoute = recovery.routeIndex
    recovery.failed(afterHealthyConnection: true, routeCount: 2)
    #expect(recovery.routeIndex == workingRoute)
    #expect(recovery.failures == 0)
    let plan = try ConnectionPlan(endpoints: ["wss://a.example", "wss://b.example"], room: "room", allowLocalInsecure: false)
    #expect(plan.retryDelay(attempt: recovery.failures, jitter: 1) <= 0.6)
    recovery.failed(afterHealthyConnection: false, routeCount: 2)
    #expect(recovery.routeIndex != workingRoute)
    #expect(recovery.failures == 1)
}
