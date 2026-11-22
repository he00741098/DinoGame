import Foundation
import Testing
@testable import MultiplayerKit

/// Optional generated fixture: no server source or campaign asset is bundled in the app.
@Test(.enabled(if: ProcessInfo.processInfo.environment["COOP_CAMPAIGN_FIXTURE"] != nil))
func actualServerCatalogAndDynamicSnapshotsDecodeForAllTwentyLevels() throws {
    let path = try #require(ProcessInfo.processInfo.environment["COOP_CAMPAIGN_FIXTURE"])
    let lines = try String(contentsOfFile: path, encoding: .utf8).split(separator: "\n")
    #expect(lines.count == Wire.levelCount + 1)
    var stream = SnapshotStream()
    guard case .welcome(let welcome) = try ServerEvent.decode(Data(lines[0].utf8)) else {
        Issue.record("Missing server welcome"); return
    }
    try stream.start(welcome)
    var kinds = Set<MechanicKind>()
    for (index, line) in lines.dropFirst().enumerated() {
        guard case .snapshot(let snapshot) = try ServerEvent.decode(Data(line.utf8)) else {
            Issue.record("Missing server snapshot"); return
        }
        #expect(snapshot.level.number == index + 1)
        let accepted = try stream.accept(snapshot, lastSentSequence: 0)
        #expect(accepted)
        kinds.formUnion(snapshot.level.mechanics.map(\.kind))
    }
    #expect(kinds == Set(MechanicKind.allCases))
    #expect(stream.level?.number == Wire.levelCount)
}

@Suite struct ChatWireTests {
    @Test func chatDecodeRejectsMismatchedWording() throws {
        let ok = Data(#"{"type":"chat","player":2,"preset":2,"text":"Jump!"}"#.utf8)
        guard case .chat(let message) = try ServerEvent.decode(ok) else { Issue.record("wrong variant"); return }
        #expect(message.player == 2)
        #expect(message.text == "Jump!")
        #expect(throws: ProtocolError.self) {
            try ServerEvent.decode(Data(#"{"type":"chat","player":2,"preset":2,"text":"hi"}"#.utf8))
        }
        #expect(throws: ProtocolError.self) {
            try ServerEvent.decode(Data(#"{"type":"chat","player":2,"preset":9,"text":"x"}"#.utf8))
        }
    }

    @Test func chatCommandEncodesPresetOnly() throws {
        let object = try JSONSerialization.jsonObject(with: ClientCommand.chat(preset: 3)) as? [String: Any]
        #expect(object?["type"] as? String == "chat")
        #expect(object?["preset"] as? Int == 3)
        #expect(object?.count == 2)
        #expect(throws: ProtocolError.self) { try ClientCommand.chat(preset: 6) }
    }
}
