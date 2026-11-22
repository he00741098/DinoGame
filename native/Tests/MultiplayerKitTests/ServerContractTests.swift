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
