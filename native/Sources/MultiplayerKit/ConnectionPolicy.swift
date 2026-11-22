import Foundation

public enum ConnectionError: Error, Equatable, Sendable {
    case invalidRoom, invalidEndpoint, insecureRemoteEndpoint, noRoutes
}

/// A route is an address for the SAME session protocol, not a separate game mode.
/// Current routes are WebSocket endpoints (LAN or hosted); a future peer adapter can
/// implement this session boundary without duplicating gameplay/lobby state.
public struct ConnectionRoute: Equatable, Sendable {
    public let url: URL

    public init(_ text: String, allowLocalInsecure: Bool) throws {
        guard var parts = URLComponents(string: text.trimmingCharacters(in: .whitespacesAndNewlines)),
              let scheme = parts.scheme?.lowercased(), ["ws", "wss"].contains(scheme),
              let host = parts.host, !host.isEmpty, parts.user == nil, parts.password == nil,
              parts.fragment == nil, parts.query == nil,
              parts.port.map({ (1...65535).contains($0) }) ?? true else { throw ConnectionError.invalidEndpoint }
        if scheme == "ws" && !(allowLocalInsecure && Self.isLocalHost(host)) {
            throw ConnectionError.insecureRemoteEndpoint
        }
        guard parts.path.isEmpty || parts.path == "/" || parts.path == "/ws" else {
            throw ConnectionError.invalidEndpoint
        }
        parts.scheme = scheme
        parts.path = "/ws"
        guard let url = parts.url else { throw ConnectionError.invalidEndpoint }
        self.url = url
    }

    private static func isLocalHost(_ raw: String) -> Bool {
        let host = raw.lowercased().trimmingCharacters(in: CharacterSet(charactersIn: "[]"))
        if host == "localhost" || host == "::1" || host.hasSuffix(".local") { return true }
        let components = host.split(separator: ".", omittingEmptySubsequences: false)
        let octets = components.compactMap { UInt8($0) }
        guard octets.count == 4, components.count == 4,
              components.allSatisfy({ $0.count == 1 || !$0.hasPrefix("0") }) else {
            return false // IPv6 LAN addressing is deferred rather than guessed by prefix.
        }
        return octets[0] == 10 || octets[0] == 127
            || (octets[0] == 192 && octets[1] == 168)
            || (octets[0] == 172 && (16...31).contains(octets[1]))
            || (octets[0] == 169 && octets[1] == 254)
    }
}

public struct ConnectionPlan: Sendable {
    public let routes: [ConnectionRoute]
    public let room: String

    public init(endpoints: [String], room: String, allowLocalInsecure: Bool) throws {
        guard (3...16).contains(room.utf8.count), room.utf8.allSatisfy({
            (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) || $0 == 45
        }) else { throw ConnectionError.invalidRoom }
        var routes: [ConnectionRoute] = []
        for text in endpoints where !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            let route = try ConnectionRoute(text, allowLocalInsecure: allowLocalInsecure)
            if !routes.contains(route) { routes.append(route) }
        }
        guard !routes.isEmpty, routes.count <= 4 else { throw ConnectionError.noRoutes }
        self.routes = routes
        self.room = room
    }

    public func route(attempt: Int) -> ConnectionRoute { routes[max(0, attempt) % routes.count] }

    /// Finite, capped backoff. Caller supplies bounded jitter; no wall clock/randomness in policy tests.
    public func retryDelay(attempt: Int, jitter: Double) -> Double {
        let base = min(8.0, pow(2.0, Double(min(4, max(0, attempt)))) * 0.5)
        return min(base * 1.2, base * (0.8 + min(1, max(0, jitter.isFinite ? jitter : 0.5)) * 0.4))
    }
}

/// Buttons are sources, not just booleans: releasing one finger cannot cancel another.
public struct InputState: Sendable {
    private var sources: [String: Action] = [:]
    private var pendingJump = false
    public private(set) var jumpID: UInt32 = 0
    private var transmittedJumpID: UInt32 = 0
    public enum Action: Sendable { case left, right, jump }
    public init() {}

    public mutating func set(_ action: Action, source: String, pressed: Bool) {
        let jumpWasHeld = sources.values.contains(.jump)
        if pressed { sources[source] = action } else { sources.removeValue(forKey: source) }
        if !jumpWasHeld && sources.values.contains(.jump) {
            pendingJump = true
            if jumpID < .max { jumpID += 1 }
        }
    }

    /// A complete tap between timer samples is retained for one outgoing frame.
    public mutating func sample() -> GameInput {
        defer { pendingJump = false }
        return GameInput(axis: (sources.values.contains(.right) ? 1 : 0) - (sources.values.contains(.left) ? 1 : 0),
                         jump: pendingJump || sources.values.contains(.jump))
    }

    /// Commit immediately before offering a frame to the transport (before any await).
    /// Clearing controls can discard a not-yet-sent edge, but never regress wire IDs.
    public mutating func markTransmitted() { transmittedJumpID = jumpID }

    public mutating func clear() {
        sources.removeAll(); pendingJump = false
        jumpID = transmittedJumpID
    }
}

/// Route selection and failure backoff are separate state. A healthy connection
/// resets failure history and is retried first, rather than carrying an old
/// outage's long delay into a new, time-limited resume window.
public struct RecoveryState: Sendable {
    public private(set) var routeIndex = 0
    public private(set) var failures = 0
    public init(routeIndex: Int = 0) { self.routeIndex = max(0, routeIndex) }

    public mutating func failed(afterHealthyConnection healthy: Bool, routeCount: Int) {
        let count = max(1, routeCount)
        routeIndex %= count
        if healthy {
            failures = 0
        } else {
            failures = min(4, failures + 1)
            routeIndex = (routeIndex + 1) % count
        }
    }
}
