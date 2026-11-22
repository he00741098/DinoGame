import Testing
@testable import MultiplayerKit

@Test func pathHintsWakeRecoveryButDuplicatesDoNotRestartSessions() {
    var policy = PathRecoveryState()
    let wifi = NetworkPathSignal(reachable: true, interface: .wifi)
    let mobile = NetworkPathSignal(reachable: true, interface: .cellular)
    let offline = NetworkPathSignal(reachable: false, interface: .other)
    let signals = [wifi, wifi, offline, offline, wifi, mobile, mobile]
    let recoveries = signals.map { policy.observe($0) }
    #expect(recoveries == [false, false, true, false, true, true, false])
}

@Test func lastHealthyFallbackCanBePreferredAcrossLifecycleRecovery() {
    var recovery = RecoveryState(routeIndex: 1)
    #expect(recovery.routeIndex == 1)
    recovery.failed(afterHealthyConnection: true, routeCount: 2)
    #expect(recovery.routeIndex == 1)
    #expect(recovery.failures == 0)
    recovery.failed(afterHealthyConnection: false, routeCount: 2)
    #expect(recovery.routeIndex == 0)
    var extreme = RecoveryState(routeIndex: .max)
    extreme.failed(afterHealthyConnection: false, routeCount: 2)
    #expect(extreme.routeIndex == 0)
}

@Test func missingInputAckHasDeadlineEvenWhileOtherTrafficIsHealthy() {
    var deadline = InputAcknowledgmentDeadline()
    let start = ContinuousClock.now
    #expect(!deadline.expired(at: start.advanced(by: .seconds(20))))
    deadline.offer(sequence: 10, at: start)
    deadline.offer(sequence: 11, at: start.advanced(by: .seconds(2)))
    deadline.acknowledge(sequence: 9)
    #expect(!deadline.expired(at: start.advanced(by: .milliseconds(2999))))
    #expect(deadline.expired(at: start.advanced(by: .seconds(3))))
    deadline.acknowledge(sequence: 10)
    #expect(!deadline.expired(at: start.advanced(by: .seconds(20))))
    deadline.offer(sequence: 12, at: start.advanced(by: .seconds(20)))
    deadline.acknowledge(sequence: 11)
    #expect(deadline.expired(at: start.advanced(by: .seconds(23))))
}
