import Foundation
import Testing
@testable import MultiplayerKit

@Test func snapshotPatchesRequireExactConnectionBaseAndPreserveNulls() throws {
    let rect: [String: Int] = ["x": 0, "y": 0, "w": 512, "h": 512]
    let snapshot: [String: Any] = ["type": "snapshot", "revision": 1, "controller": NSNull(), "acks": [], "campaign_complete": false,
        "level": ["number": 1, "title": "Test", "hint": "", "mechanics": [], "width": 24576, "height": 14336, "solids": [], "gate": rect, "key": rect, "exit": rect],
        "world": ["tick": 1, "level_start_tick": 0, "players": [], "objects": [], "deaths": 0, "checkpoint": NSNull(), "key_collected": false, "completed": false]]
    let full = try JSONSerialization.data(withJSONObject: snapshot)
    let patch = try JSONSerialization.data(withJSONObject: ["type": "patch", "revision": 1, "base_tick": 1, "changes": ["world": ["tick": 7, "checkpoint": NSNull()]]])
    var wire = SnapshotWireDecoder()
    _ = try wire.decode(full)
    guard case .snapshot(let decoded) = try wire.decode(patch) else { Issue.record("No snapshot"); return }
    #expect(decoded.world.tick == 7 && decoded.world.checkpoint == nil)
    do { _ = try wire.decode(patch); Issue.record("Duplicate base accepted") } catch {}
    var fresh = SnapshotWireDecoder()
    do { _ = try fresh.decode(patch); Issue.record("Patch before base accepted") } catch {}
    _ = try wire.decode(full) // Full frame restores a base on any revision.
}
