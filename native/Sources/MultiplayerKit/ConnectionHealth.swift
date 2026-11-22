import Foundation

/// Reachability is a hint, never proof that a server is reachable (or unreachable).
/// Keep the actual session/watchdog/retry policy authoritative for connectivity.
public struct NetworkPathSignal: Equatable, Sendable {
    public enum Interface: Sendable { case wifi, cellular, wired, other }
    public let reachable: Bool
    public let interface: Interface
    public init(reachable: Bool, interface: Interface) {
        self.reachable = reachable
        self.interface = interface
    }
}

public struct PathRecoveryState: Sendable {
    private var previous: NetworkPathSignal?
    public init() {}

    /// Initial/duplicate notifications are not outages. A transition can wake
    /// recovery immediately instead of waiting for the five-second socket timer.
    public mutating func observe(_ signal: NetworkPathSignal) -> Bool {
        defer { previous = signal }
        guard let previous else { return false }
        return previous.reachable != signal.reachable
            || (signal.reachable && previous.interface != signal.interface)
    }
}

/// Fresh snapshots are insufficient if the server never acknowledges an input.
/// Start once per synchronization attempt; repeated heartbeats cannot extend it.
public struct InputAcknowledgmentDeadline: Sendable {
    private var started: ContinuousClock.Instant?
    private var target: UInt32?
    public init() {}
    public mutating func offer(sequence: UInt32, at instant: ContinuousClock.Instant) {
        if started == nil { started = instant; target = sequence }
    }
    public mutating func acknowledge(sequence: UInt32) {
        if let target, sequence >= target { clear() }
    }
    public mutating func clear() { started = nil; target = nil }
    public func expired(at instant: ContinuousClock.Instant) -> Bool {
        started.map { $0.duration(to: instant) >= .seconds(3) } ?? false
    }
}
