import Foundation
import Testing
@testable import MultiplayerKit

@Test func routesRejectInsecureInternetAndCredentials() throws {
    for text in ["ws://example.com", "http://localhost:3000", "wss://user:password@example.com",
                 "wss://example.com/ws?token=secret", "wss://example.com/other", "ws://192.168.1.1.evil.com",
                 "ws://172.016.1.1"] {
        #expect(throws: (any Error).self) { try ConnectionRoute(text, allowLocalInsecure: true) }
    }
    #expect(throws: ConnectionError.insecureRemoteEndpoint) {
        try ConnectionRoute("ws://127.0.0.1:3000", allowLocalInsecure: false)
    }
    for text in ["ws://127.0.0.1:3000", "ws://192.168.1.10:3000", "ws://host.local:3000", "wss://example.com"] {
        #expect(try ConnectionRoute(text, allowLocalInsecure: true).url.path == "/ws")
    }
}

@Test func routingIsOneBoundedPlanWithCappedBackoff() throws {
    let plan = try ConnectionPlan(endpoints: ["wss://a.example", "wss://b.example", "wss://a.example/"],
                                  room: "test-1", allowLocalInsecure: false)
    #expect(plan.routes.count == 2)
    #expect(plan.route(attempt: 2) == plan.routes[0])
    for attempt in [0, 1, 20, Int.max] {
        for jitter in [-1.0, 0, 1, .infinity, .nan] {
            let delay = plan.retryDelay(attempt: attempt, jitter: jitter)
            #expect(delay.isFinite && delay >= 0.4 && delay <= 9.6)
        }
    }
    #expect(throws: ConnectionError.invalidRoom) {
        try ConnectionPlan(endpoints: ["wss://a.example"], room: "../secret", allowLocalInsecure: false)
    }
    #expect(throws: ConnectionError.noRoutes) {
        try ConnectionPlan(endpoints: [""], room: "valid", allowLocalInsecure: false)
    }
}

@Test func shortTapSurvivesSamplingWithMonotonicEdgeID() {
    var input = InputState()
    input.set(.jump, source: "finger1", pressed: true)
    input.set(.jump, source: "finger1", pressed: false)
    #expect(input.jumpID == 1)
    #expect(input.sample().jump)
    #expect(!input.sample().jump)
    input.set(.jump, source: "finger2", pressed: true)
    input.set(.jump, source: "finger2", pressed: true)
    #expect(input.jumpID == 2)
    input.clear()
    #expect(input.sample() == GameInput())
}

@Test func inputSourcesDoNotCancelEachOtherAndOppositesAreNeutral() {
    var input = InputState()
    input.set(.left, source: "one", pressed: true)
    input.set(.left, source: "two", pressed: true)
    input.set(.left, source: "one", pressed: false)
    #expect(input.sample().axis == -1)
    input.set(.right, source: "three", pressed: true)
    #expect(input.sample().axis == 0)
    input.clear()
    #expect(input.sample().axis == 0)
}

let fixtureLevel = """
{"number":1,"title":"Together","hint":"Reach the key together","mechanics":[],
 "width":24576,"height":14336,"solids":[{"x":0,"y":12288,"w":24576,"h":2048}],
 "gate":{"x":9216,"y":9216,"w":3072,"h":3072},"key":{"x":10240,"y":8192,"w":512,"h":512},
 "exit":{"x":22528,"y":10240,"w":1024,"h":2048}}
"""

func welcomeData(version: Int = 3) -> Data {
    Data("""
    {"type":"welcome","protocol":\(version),"ruleset":"coop-campaign-1","player":1,"revision":0,"controller":1,
     "tick_hz":60,"snapshot_hz":20,"resume_token":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","resumed":false,
     "level":\(fixtureLevel)}
    """.utf8)
}

func snapshotData(tick: Int = 1, ack: Int = 0, x: Int = 2048, duplicate: Bool = false) -> Data {
    let player = """
    {"id":1,"x":\(x),"y":11392,"vx":0,"vy":0,"grounded":true,"jump_held":false}
    """
    return Data("""
    {"type":"snapshot","level":\(fixtureLevel),"revision":0,"controller":1,"campaign_complete":false,
    "world":{"tick":\(tick),"players":[\(player)\(duplicate ? "," + player : "")],
    "objects":[],"deaths":0,"checkpoint":null,"level_start_tick":0,
    "key_collected":false,"completed":false},"acks":[{"player":1,"seq":\(ack)}]}
    """.utf8)
}

@Test func protocolRejectsMismatchOversizeAndUnknownMessages() {
    #expect(throws: ProtocolError.incompatible) { try ServerEvent.decode(welcomeData(version: 999)) }
    #expect(throws: ProtocolError.oversized) { try ServerEvent.decode(Data(repeating: 32, count: 65_537)) }
    #expect(throws: ProtocolError.unknownMessage) { try ServerEvent.decode(Data("{\"type\":\"other\"}".utf8)) }
}

@Test func streamChecksOrderingTickFreshnessBoundsAndAcks() throws {
    var stream = SnapshotStream()
    guard case .snapshot(let snapshot) = try ServerEvent.decode(snapshotData()),
          case .welcome(let welcome) = try ServerEvent.decode(welcomeData()) else { return }
    #expect(throws: ProtocolError.invalidOrder) { try stream.accept(snapshot, lastSentSequence: 0) }
    try stream.start(welcome)
    #expect(try stream.accept(snapshot, lastSentSequence: 0))
    #expect(try !stream.accept(snapshot, lastSentSequence: 0))
    for data in [snapshotData(tick: 2, ack: 2), snapshotData(tick: 2, x: -1), snapshotData(tick: 2, duplicate: true)] {
        guard case .snapshot(let bad) = try ServerEvent.decode(data) else { continue }
        #expect(throws: ProtocolError.invalidSnapshot) { try stream.accept(bad, lastSentSequence: 1) }
    }
    guard case .snapshot(let next) = try ServerEvent.decode(snapshotData(tick: 3, ack: 1)),
          case .snapshot(let regression) = try ServerEvent.decode(snapshotData(tick: 4, ack: 0)) else { return }
    #expect(try stream.accept(next, lastSentSequence: 1))
    #expect(throws: ProtocolError.invalidSnapshot) { try stream.accept(regression, lastSentSequence: 1) }
}

@Test func encoderProducesOnlyProtocolCommands() throws {
    let hello = try JSONSerialization.jsonObject(with: ClientCommand.hello(room: "test")) as! [String: Any]
    #expect(hello["protocol"] as? Int == 3)
    #expect(hello["ruleset"] as? String == "coop-campaign-1")
    #expect(hello["resume_token"] == nil)
    let command = try JSONSerialization.jsonObject(with: ClientCommand.input(seq: 1, input: GameInput(axis: 2), jumpID: 4, revision: 7)) as! [String: Any]
    #expect(command["revision"] as? Int == 7)
    #expect(command["jump_id"] as? Int == 4)
    #expect(command["x"] == nil)
    #expect((command["input"] as? [String: Any])?["axis"] as? Int == 1)
}
