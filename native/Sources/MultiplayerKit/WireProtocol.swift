// Original native client protocol implementation; no Rust or legacy code is linked.
import Foundation

public enum Wire {
    public static let version = 3
    public static let game = "coop-sandbox"
    public static let ruleset = "coop-campaign-3"
    public static let levelCount = 28
    public static let playerWidth = 720
    public static let playerHeight = 896
}

public struct GameInput: Codable, Equatable, Sendable {
    public var axis: Int
    public var jump: Bool
    public init(axis: Int = 0, jump: Bool = false) {
        self.axis = min(1, max(-1, axis))
        self.jump = jump
    }
}

public struct GameRect: Codable, Equatable, Sendable {
    public let x: Int
    public let y: Int
    public let w: Int
    public let h: Int
    public init(x: Int, y: Int, w: Int, h: Int) {
        self.x = x; self.y = y; self.w = w; self.h = h
    }
    public var isSafe: Bool {
        (0...1_000_000).contains(x) && (0...1_000_000).contains(y)
            && (1...1_000_000).contains(w) && (1...1_000_000).contains(h)
    }
}

public struct GameLevel: Codable, Equatable, Sendable {
    public let tether_length: Int?
    public var tetherLength: Int { tether_length ?? 0 }
    public let required_players: Int?
    public var minimumPlayers: Int { required_players ?? 2 }
    public let number: Int
    public let title: String
    public let hint: String
    public let mechanics: [GameMechanic]
    public let width: Int
    public let height: Int
    public let solids: [GameRect]
    public let gate: GameRect
    public let key: GameRect
    public let exit: GameRect

    public var isSafe: Bool {
        guard (16_384...262_144).contains(width), (14_336...131_072).contains(height),
              tetherLength == 0 || (2048...12288).contains(tetherLength),
              (0...Wire.levelCount).contains(number), (2...4).contains(minimumPlayers), title.utf8.count <= 128, hint.utf8.count <= 1024,
              solids.count <= 128, mechanics.count <= 64,
              Set(mechanics.map(\.id)).count == mechanics.count else { return false }
        return (solids + [gate, key, exit]).allSatisfy(contains) && mechanics.allSatisfy {
            contains($0.rect) && ($0.kind == .fallingRock ? (0...(height - $0.rect.y - $0.rect.h)).contains($0.travel) : (-8192...8192).contains($0.travel))
                && $0.period <= 3600
                && ($0.kind != .movingPlatform || (0...(width - $0.rect.w)).contains($0.rect.x + $0.travel))
        }
    }

    public func contains(_ rect: GameRect) -> Bool {
        rect.isSafe && rect.x + rect.w <= width && rect.y + rect.h <= height
    }
}

public enum MechanicKind: String, Codable, CaseIterable, Sendable {
    case plate, pressureGate = "pressure_gate", timedGate = "timed_gate", oneWay = "one_way"
    case movingPlatform = "moving_platform", conveyor, ice, spring, hazard, checkpoint, crumble
    case lowGravity = "low_gravity", wind
    case rope, fallingRock = "falling_rock"
}

public struct GameMechanic: Codable, Equatable, Sendable {
    public let id: UInt32
    public let kind: MechanicKind
    public let rect: GameRect
    public let travel: Int
    public let period: UInt32
}

public struct MechanicState: Codable, Equatable, Sendable {
    public let id: UInt32
    public let rect: GameRect
    public let active: Bool
    public let timer: UInt32
}

public struct GamePlayer: Codable, Equatable, Sendable {
    public let id: UInt32
    public let x: Int
    public let y: Int
    public let vx: Int
    public let vy: Int
    public let grounded: Bool
    public let jump_held: Bool
}

public struct GameWorld: Codable, Equatable, Sendable {
    public let tick: UInt64
    public let players: [GamePlayer]
    public let key_collected: Bool
    public let completed: Bool
    public let objects: [MechanicState]
    public let deaths: UInt32
    public let checkpoint: GameRect?
    public let level_start_tick: UInt64
}

public struct InputAck: Codable, Equatable, Sendable {
    public let player: UInt32
    public let seq: UInt32
}

public struct Welcome: Decodable, Sendable {
    public let `protocol`: Int
    public let ruleset: String
    public let player: UInt32
    public let tick_hz: Int
    public let snapshot_hz: Int
    public let resume_token: String
    public let resumed: Bool
    public let level: GameLevel
    public let revision: UInt32
    public let controller: UInt32?
}

public struct Snapshot: Decodable, Sendable {
    public let level: GameLevel
    public let revision: UInt32
    public let controller: UInt32?
    public let campaign_complete: Bool
    public let world: GameWorld
    public let acks: [InputAck]
}

public enum ProtocolError: Error, Equatable, Sendable {
    case oversized, unknownMessage, incompatible, invalidLevel, invalidSnapshot, invalidOrder, invalidInput
}

public enum ServerEvent: Sendable {
    case welcome(Welcome)
    case snapshot(Snapshot)
    case rejection(code: String, message: String)

    public static func decode(_ data: Data) throws -> Self {
        guard data.count <= 65_536 else { throw ProtocolError.oversized }
        struct Tag: Decodable { let type: String }
        let decoder = JSONDecoder()
        switch try decoder.decode(Tag.self, from: data).type {
        case "welcome":
            let value = try decoder.decode(Welcome.self, from: data)
            guard value.protocol == Wire.version, value.ruleset == Wire.ruleset,
                  value.player > 0, value.tick_hz == 60, value.snapshot_hz == 20,
                  (32...128).contains(value.resume_token.utf8.count) else {
                throw ProtocolError.incompatible
            }
            guard value.level.isSafe else { throw ProtocolError.invalidLevel }
            guard value.controller != 0 else { throw ProtocolError.invalidSnapshot }
            return .welcome(value)
        case "snapshot":
            let value = try decoder.decode(Snapshot.self, from: data)
            guard value.level.isSafe else { throw ProtocolError.invalidLevel }
            return .snapshot(value)
        case "error":
            struct Rejection: Decodable { let code: String; let message: String }
            let value = try decoder.decode(Rejection.self, from: data)
            return .rejection(code: value.code, message: value.message)
        default: throw ProtocolError.unknownMessage
        }
    }
}

public enum ClientCommand {
    public static func hello(room: String, resumeToken: String? = nil, patches: Bool = false) throws -> Data {
        struct Hello: Encodable {
            let type = "hello"
            let `protocol` = Wire.version
            let game = Wire.game
            let ruleset = Wire.ruleset
            let room: String
            let resume_token: String?
            let patches: Bool?
        }
        return try JSONEncoder().encode(Hello(room: room, resume_token: resumeToken, patches: patches ? true : nil))
    }

    public static func input(seq: UInt32, input: GameInput, jumpID: UInt32, revision: UInt32) throws -> Data {
        guard seq > 0, (-1...1).contains(input.axis) else { throw ProtocolError.invalidInput }
        struct Command: Encodable { let type = "input"; let seq: UInt32; let input: GameInput; let jump_id: UInt32; let revision: UInt32 }
        return try JSONEncoder().encode(Command(seq: seq, input: input, jump_id: jumpID, revision: revision))
    }

    public static func control(action: CampaignAction, seq: UInt32, revision: UInt32) throws -> Data {
        guard seq > 0 else { throw ProtocolError.invalidInput }
        struct Command: Encodable { let type: CampaignAction; let seq: UInt32; let revision: UInt32 }
        return try JSONEncoder().encode(Command(type: action, seq: seq, revision: revision))
    }
}

public enum CampaignAction: String, Codable, Sendable { case restart, next }

/// Validates a complete authoritative stream before presentation sees it.
public struct SnapshotStream: Sendable {
    public private(set) var welcome: Welcome?
    public private(set) var lastTick: UInt64?
    public private(set) var revision: UInt32?
    public private(set) var level: GameLevel?
    private var lastAck: UInt32 = 0
    public init() {}

    public mutating func start(_ welcome: Welcome) throws {
        guard self.welcome == nil else { throw ProtocolError.invalidOrder }
        self.welcome = welcome
        revision = welcome.revision
        level = welcome.level
    }

    /// False means duplicate/out-of-order snapshot: do not redraw or refresh its freshness timer.
    public mutating func accept(_ snapshot: Snapshot, lastSentSequence: UInt32) throws -> Bool {
        guard let welcome else { throw ProtocolError.invalidOrder }
        let world = snapshot.world
        let level = snapshot.level
        guard level.isSafe else { throw ProtocolError.invalidLevel }
        guard let revision, snapshot.revision >= revision,
              snapshot.revision != revision || self.level == level else { throw ProtocolError.invalidOrder }
        guard (1...4).contains(world.players.count),
              Set(world.players.map(\.id)).count == world.players.count,
              world.players.contains(where: { $0.id == welcome.player }),
              snapshot.acks.count <= world.players.count,
              Set(snapshot.acks.map(\.player)).count == snapshot.acks.count,
              Set(snapshot.acks.map(\.player)).isSubset(of: Set(world.players.map(\.id))),
              (snapshot.acks.first(where: { $0.player == welcome.player })?.seq ?? 0) <= lastSentSequence,
              world.players.allSatisfy({ player in
                  player.id > 0 && player.x >= 0 && player.y >= 0
                      && player.x <= level.width - Wire.playerWidth
                      && player.y <= level.height - Wire.playerHeight
                      && (-4096...4096).contains(player.vx) && (-4096...4096).contains(player.vy)
               }),
              snapshot.controller.map({ id in world.players.contains(where: { $0.id == id }) }) ?? true,
               !snapshot.campaign_complete || (level.number == Wire.levelCount && world.completed),
              world.level_start_tick <= world.tick,
              world.checkpoint.map({ rect in level.mechanics.contains { $0.kind == .checkpoint && $0.rect == rect } }) ?? true,
              world.objects.count == level.mechanics.count,
              Set(world.objects.map(\.id)).count == world.objects.count else { throw ProtocolError.invalidSnapshot }
        let mechanics = Dictionary(uniqueKeysWithValues: level.mechanics.map { ($0.id, $0) })
        for object in world.objects {
            guard let mechanic = mechanics[object.id], level.contains(object.rect), object.timer <= 3600,
                  object.rect.w == mechanic.rect.w, object.rect.h == mechanic.rect.h else {
                throw ProtocolError.invalidSnapshot
            }
            if mechanic.kind == .movingPlatform {
                let end = mechanic.rect.x + mechanic.travel
                guard object.rect.y == mechanic.rect.y,
                      (min(mechanic.rect.x, end)...max(mechanic.rect.x, end)).contains(object.rect.x) else {
                    throw ProtocolError.invalidSnapshot
                }
            } else if mechanic.kind == .fallingRock {
                guard object.rect.x == mechanic.rect.x,
                      (mechanic.rect.y...(mechanic.rect.y + mechanic.travel)).contains(object.rect.y) else { throw ProtocolError.invalidSnapshot }
            } else if object.rect != mechanic.rect { throw ProtocolError.invalidSnapshot }
        }
        if let lastTick, world.tick < lastTick || (world.tick == lastTick && snapshot.revision == revision) { return false }
        let ack = snapshot.acks.first(where: { $0.player == welcome.player })?.seq ?? 0
        guard ack >= lastAck else { throw ProtocolError.invalidSnapshot }
        lastTick = world.tick
        lastAck = ack
        self.revision = snapshot.revision
        self.level = level
        return true
    }
}
