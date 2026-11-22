import Foundation

/// Connection-local reconstruction. The transport is ordered; patches must name
/// the exact last received base. Reset on every connection, never across routes.
/// Arrays replace atomically and JSON null remains a value.
public struct SnapshotWireDecoder {
    private var base: [String: Any]?
    public init() {}
    public mutating func decode(_ data: Data) throws -> ServerEvent {
        guard data.count <= 65_536 else { throw ProtocolError.oversized }
        guard let message = try JSONSerialization.jsonObject(with: data) as? [String: Any] else { throw ProtocolError.unknownMessage }
        var complete = data
        if message["type"] as? String == "patch" {
            guard let base, let world = base["world"] as? [String: Any],
                  (message["base_tick"] as? NSNumber) == (world["tick"] as? NSNumber),
                  (message["revision"] as? NSNumber) == (base["revision"] as? NSNumber),
                  let changes = message["changes"] as? [String: Any],
                  changes["type"] == nil, changes["level"] == nil, changes["revision"] == nil else { throw ProtocolError.invalidOrder }
            let merged = try Self.merge(base, changes, depth: 0)
            complete = try JSONSerialization.data(withJSONObject: merged)
            guard complete.count <= 65_536 else { throw ProtocolError.oversized }
        }
        let event = try ServerEvent.decode(complete)
        if case .snapshot = event {
            base = try JSONSerialization.jsonObject(with: complete) as? [String: Any]
        }
        return event
    }
    private static func merge(_ base: [String: Any], _ changes: [String: Any], depth: Int) throws -> [String: Any] {
        guard depth < 16 else { throw ProtocolError.invalidSnapshot }
        var result = base
        for (key, value) in changes {
            guard base[key] != nil else { throw ProtocolError.invalidSnapshot }
            if let object = value as? [String: Any], let original = base[key] as? [String: Any] {
                result[key] = try merge(original, object, depth: depth + 1)
            } else { result[key] = value }
        }
        return result
    }
}
