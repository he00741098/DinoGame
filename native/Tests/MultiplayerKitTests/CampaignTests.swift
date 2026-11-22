import Foundation
import Testing
@testable import MultiplayerKit

private func editingJSON(_ data: Data, _ edit: (inout [String: Any]) -> Void) throws -> Data {
    var value = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
    edit(&value)
    return try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
}

private func snapshot(_ data: Data) throws -> Snapshot {
    guard case .snapshot(let value) = try ServerEvent.decode(data) else { throw ProtocolError.unknownMessage }
    return value
}

private func campaignStream() throws -> SnapshotStream {
    var stream = SnapshotStream()
    guard case .welcome(let welcome) = try ServerEvent.decode(welcomeData()) else { throw ProtocolError.unknownMessage }
    try stream.start(welcome)
    return stream
}

private let mechanicRect: [String: Int] = ["x": 4096, "y": 10000, "w": 2048, "h": 256]

private func mechanicSnapshot(kind: MechanicKind, travel: Int = 0) throws -> Data {
    try editingJSON(snapshotData()) { value in
        var level = value["level"] as! [String: Any]
        level["mechanics"] = [["id": 7, "kind": kind.rawValue, "rect": mechanicRect, "travel": travel, "period": 180]]
        value["level"] = level
        var world = value["world"] as! [String: Any]
        world["objects"] = [["id": 7, "rect": mechanicRect, "active": true, "timer": 0]]
        value["world"] = world
        value["revision"] = 1
    }
}

@Test func campaignControlsHaveRevisionAndNoClientAuthority() throws {
    for action in [CampaignAction.restart, .next] {
        let command = try #require(JSONSerialization.jsonObject(with: ClientCommand.control(action: action, seq: 5, revision: 2)) as? [String: Any])
        #expect(command["type"] as? String == action.rawValue)
        #expect(command["seq"] as? Int == 5)
        #expect(command["revision"] as? Int == 2)
        #expect(Set(command.keys) == ["type", "seq", "revision"])
    }
    #expect(throws: ProtocolError.invalidInput) { try ClientCommand.control(action: .next, seq: 0, revision: 1) }
    let hello = try #require(JSONSerialization.jsonObject(with: ClientCommand.hello(room: "campaign", resumeToken: "secret")) as? [String: Any])
    #expect(hello["game"] as? String == "coop-sandbox")
    #expect(hello["resume_token"] as? String == "secret")
}

@Test func campaignRulesetIsRequiredAndLegacyLevelZeroIsBounded() throws {
    let oldRuleset = String(decoding: welcomeData(), as: UTF8.self)
        .replacingOccurrences(of: "coop-campaign-3", with: "coop-sandbox-1")
    #expect(throws: ProtocolError.incompatible) { try ServerEvent.decode(Data(oldRuleset.utf8)) }
    let legacy = try editingJSON(welcomeData()) { value in
        var level = value["level"] as! [String: Any]
        level["number"] = 0; value["level"] = level
    }
    guard case .welcome(let welcome) = try ServerEvent.decode(legacy) else { throw ProtocolError.unknownMessage }
    #expect(welcome.level.number == 0)
    #expect(welcome.level.mechanics.isEmpty)
}

@Test func fallingRockCanOnlyMoveDownItsDeclaredShaft() throws {
    let source = try mechanicSnapshot(kind: .fallingRock, travel: 2048)
    let moved = try editingJSON(source) { value in
        var world = value["world"] as! [String: Any]
        var rect = mechanicRect; rect["y"] = 11024
        world["objects"] = [["id": 7, "rect": rect, "active": true, "timer": 100]]
        value["world"] = world
    }
    var stream = try campaignStream()
    let accepted = try stream.accept(snapshot(moved), lastSentSequence: 0)
    #expect(accepted)
    let invalid = try editingJSON(source) { value in
        var world = value["world"] as! [String: Any]
        var rect = mechanicRect; rect["y"] = 12049
        world["objects"] = [["id": 7, "rect": rect, "active": true, "timer": 100]]
        value["world"] = world
    }
    var other = try campaignStream()
    #expect(throws: ProtocolError.invalidSnapshot) { try other.accept(snapshot(invalid), lastSentSequence: 0) }
}

@Test func latestMailboxCanSkipLevelsAndTransitionAtSameTick() throws {
    var stream = try campaignStream()
    #expect(try stream.accept(snapshot(snapshotData(tick: 30, ack: 4)), lastSentSequence: 4))
    let transition = try editingJSON(snapshotData(tick: 30, ack: 4, x: 28000)) { value in
        value["revision"] = 3
        var level = value["level"] as! [String: Any]
        level["number"] = 4
        level["title"] = "Platforms"
        level["width"] = 32768
        value["level"] = level
        var world = value["world"] as! [String: Any]
        world["level_start_tick"] = 30
        value["world"] = world
    }
    // Bounds come from this snapshot's full level, not the welcome's narrower level.
    #expect(try stream.accept(snapshot(transition), lastSentSequence: 4))
    #expect(stream.level?.number == 4)
    #expect(stream.revision == 3)
    #expect(try !stream.accept(snapshot(transition), lastSentSequence: 4))
    let staleTick = try editingJSON(transition) { value in
        value["revision"] = 4
        var world = value["world"] as! [String: Any]
        world["tick"] = 29; world["level_start_tick"] = 29
        value["world"] = world
    }
    #expect(try !stream.accept(snapshot(staleTick), lastSentSequence: 4))
    #expect(stream.revision == 3)
    #expect(throws: ProtocolError.invalidOrder) { try stream.accept(snapshot(snapshotData(tick: 31, ack: 4)), lastSentSequence: 4) }
    let regressedAck = try editingJSON(transition) { $0["acks"] = [["player": 1, "seq": 3]]; $0["revision"] = 4 }
    #expect(throws: ProtocolError.invalidSnapshot) { try stream.accept(snapshot(regressedAck), lastSentSequence: 4) }
}

@Test func sameRevisionCannotMutateLevelGeometryOrMetadata() throws {
    var stream = try campaignStream()
    for field in ["width", "title", "number"] {
        let changed = try editingJSON(snapshotData()) { value in
            var level = value["level"] as! [String: Any]
            switch field {
            case "width": level[field] = 32768
            case "title": level[field] = "Imposter"
            default: level[field] = 2
            }
            value["level"] = level
        }
        #expect(throws: ProtocolError.invalidOrder) { try stream.accept(snapshot(changed), lastSentSequence: 0) }
        #expect(stream.lastTick == nil)
    }
}

@Test func allMechanicKindsDecodeAndUseAuthoritativeObjects() throws {
    for kind in MechanicKind.allCases {
        var stream = try campaignStream()
        let data = try mechanicSnapshot(kind: kind, travel: kind == .movingPlatform ? 4096 : 0)
        let value = try snapshot(data)
        #expect(value.level.mechanics.first?.kind == kind)
        #expect(try stream.accept(value, lastSentSequence: 0))
    }
}

@Test func hostileCampaignLevelAndMechanicBoundsAreRejected() throws {
    let source = try mechanicSnapshot(kind: .movingPlatform, travel: 4096)
    for (field, replacement): (String, Any) in [
        ("number", Wire.levelCount + 1), ("number", -1), ("title", String(repeating: "x", count: 129)),
        ("hint", String(repeating: "x", count: 1025)), ("width", Int.max),
    ] {
        let changed = try editingJSON(source) { value in
            var level = value["level"] as! [String: Any]
            level[field] = replacement; value["level"] = level
        }
        #expect(throws: ProtocolError.invalidLevel) { try ServerEvent.decode(changed) }
    }
    for (field, replacement): (String, Any) in [
        ("travel", Int.max), ("travel", -8193), ("period", 3601),
        ("rect", ["x": Int.max, "y": 0, "w": 1, "h": 1]),
        ("rect", ["x": 20000, "y": 10000, "w": 2048, "h": 256]),
    ] {
        let changed = try editingJSON(source) { value in
            var level = value["level"] as! [String: Any]
            var mechanics = level["mechanics"] as! [[String: Any]]
            mechanics[0][field] = replacement; level["mechanics"] = mechanics; value["level"] = level
        }
        #expect(throws: ProtocolError.invalidLevel) { try ServerEvent.decode(changed) }
    }
    for count in [2, 65] {
        let changed = try editingJSON(source) { value in
            var level = value["level"] as! [String: Any]
            let mechanics = level["mechanics"] as! [[String: Any]]
            level["mechanics"] = Array(repeating: mechanics[0], count: count); value["level"] = level
        }
        #expect(throws: ProtocolError.invalidLevel) { try ServerEvent.decode(changed) }
    }
    let unknown = String(decoding: source, as: UTF8.self).replacingOccurrences(of: "moving_platform", with: "teleport")
    #expect(throws: (any Error).self) { try ServerEvent.decode(Data(unknown.utf8)) }
}

@Test func dynamicStateCannotInventObjectsOrChangeCollisionGeometry() throws {
    for kind in [MechanicKind.ice, .movingPlatform] {
        let source = try mechanicSnapshot(kind: kind, travel: kind == .movingPlatform ? 4096 : 0)
        for (field, replacement): (String, Any) in [
            ("id", 99), ("timer", 3601),
            ("rect", ["x": 20000, "y": 10000, "w": 2048, "h": 256]),
            ("rect", ["x": 4096, "y": 10000, "w": 4096, "h": 256]),
            ("rect", ["x": 4096, "y": 9999, "w": 2048, "h": 256]),
        ] {
            var stream = try campaignStream()
            let changed = try editingJSON(source) { value in
                var world = value["world"] as! [String: Any]
                var objects = world["objects"] as! [[String: Any]]
                objects[0][field] = replacement; world["objects"] = objects; value["world"] = world
            }
            #expect(throws: ProtocolError.invalidSnapshot) { try stream.accept(snapshot(changed), lastSentSequence: 0) }
            #expect(stream.lastTick == nil)
        }
    }
    let source = try mechanicSnapshot(kind: .movingPlatform, travel: -2048)
    let moved = try editingJSON(source) { value in
        var world = value["world"] as! [String: Any]
        world["objects"] = [["id": 7, "rect": ["x": 3072, "y": 10000, "w": 2048, "h": 256], "active": true, "timer": 10]]
        value["world"] = world
    }
    var stream = try campaignStream()
    #expect(try stream.accept(snapshot(moved), lastSentSequence: 0))
}

@Test func campaignCompletionAndCheckpointMustMatchLevel() throws {
    for data in [
        try editingJSON(snapshotData()) { $0["campaign_complete"] = true },
        try editingJSON(snapshotData()) { value in
            var world = value["world"] as! [String: Any]
            world["checkpoint"] = mechanicRect; value["world"] = world
        },
        try editingJSON(snapshotData()) { value in
            var world = value["world"] as! [String: Any]
            world["level_start_tick"] = 10; value["world"] = world
        },
    ] {
        var stream = try campaignStream()
        #expect(throws: ProtocolError.invalidSnapshot) { try stream.accept(snapshot(data), lastSentSequence: 0) }
    }
    var stream = try campaignStream()
    let final = try editingJSON(snapshotData()) { value in
        value["campaign_complete"] = true; value["revision"] = 19
        var level = value["level"] as! [String: Any]; level["number"] = Wire.levelCount; value["level"] = level
        var world = value["world"] as! [String: Any]; world["completed"] = true; value["world"] = world
    }
    #expect(try stream.accept(snapshot(final), lastSentSequence: 0))
}
