import Foundation

/// Memory-only bearer credentials scoped to an exact route AND room. Never forward
/// a primary server's secret to an unrelated fallback, URL, log, or preferences file.
public struct ResumeStore: Sendable {
    private struct Key: Hashable, Sendable { let url: URL; let room: String }
    private var tokens: [Key: String] = [:]
    public init() {}
    public func token(for route: ConnectionRoute, room: String) -> String? {
        tokens[Key(url: route.url, room: room)]
    }
    public mutating func save(_ token: String, for route: ConnectionRoute, room: String) {
        let key = Key(url: route.url, room: room)
        if tokens[key] == nil && tokens.count >= 4 { tokens.removeAll() }
        tokens[key] = token
    }
    public mutating func forget(_ route: ConnectionRoute, room: String) {
        tokens.removeValue(forKey: Key(url: route.url, room: room))
    }
    public mutating func clear() { tokens.removeAll() }
}
