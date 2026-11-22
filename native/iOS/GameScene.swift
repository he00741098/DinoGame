import SpriteKit
import SwiftUI

/// GPU-backed presentation only. No physics or authority is implemented in SpriteKit.
@MainActor
final class GameScene: SKScene {
    private var currentLevel: GameLevel?
    private var players: [UInt32: SKShapeNode] = [:]
    private var objects: [UInt32: SKShapeNode] = [:]
    private var rockWarnings: [UInt32: SKShapeNode] = [:]
    private var gate: SKShapeNode?
    private var key: SKShapeNode?
    private var exit: SKShapeNode?
    private var entered: Set<UInt32> = []
    private var levelStartTick: UInt64?
    private var lastDeaths: UInt32?
    private var motion = UIAccessibility.isReduceMotionEnabled
        ? MotionTimeline(extrapolationCap: 0, smoothingDuration: 0)
        : MotionTimeline()
    private var roster: [UInt32] = []
    private let tether = SKShapeNode()
    private let followCamera = SKCameraNode()
    private let keyMarker: SKLabelNode = {
        let label = SKLabelNode(fontNamed: "AvenirNext-Bold")
        label.fontSize = 12; label.fontColor = .systemYellow; label.zPosition = 10_000
        label.verticalAlignmentMode = .center
        return label
    }()
    private var localPlayer: UInt32?
    private var lastChatAt: Double?
    private var markers: [UInt32: SKLabelNode] = [:]
    private let palette: [UIColor] = [.systemMint, .systemCyan, .systemOrange, .systemPurple]

    override init(size: CGSize) {
        super.init(size: size)
        scaleMode = .resizeFill
        backgroundColor = UIColor(red: 0.06, green: 0.10, blue: 0.17, alpha: 1)
        addChild(followCamera)
        camera = followCamera
    }
    required init?(coder: NSCoder) { fatalError("Programmatic scene only") }

    func present(level: GameLevel?, world: GameWorld?, localID: UInt32?, chat: ChatEntry? = nil) {
        guard let level, let world else {
            removeAllChildren(); players.removeAll(); objects.removeAll(); rockWarnings.removeAll(); markers.removeAll(); currentLevel = nil
            followCamera.removeAllChildren(); addChild(followCamera)
            gate = nil; key = nil; exit = nil
            entered.removeAll(); levelStartTick = nil
            motion.reset(); lastDeaths = nil; roster = []; lastChatAt = nil
            return
        }
        if currentLevel != level || levelStartTick != world.level_start_tick {
            removeAllChildren(); players.removeAll(); objects.removeAll(); rockWarnings.removeAll(); markers.removeAll()
            followCamera.removeAllChildren(); addChild(followCamera)
            currentLevel = level
            levelStartTick = world.level_start_tick; entered.removeAll()
            motion.reset(); lastChatAt = nil
            for rect in level.solids { addChild(node(rect, color: UIColor(red: 0.24, green: 0.34, blue: 0.44, alpha: 1))) }
            gate = node(level.gate, color: .systemIndigo)
            key = node(level.key, color: .systemYellow)
            exit = node(level.exit, color: .darkGray)
            gate?.zPosition = 1; exit?.zPosition = 2; key?.zPosition = 5
            for item in [gate, key, exit].compactMap({ $0 }) { addChild(item) }
            addLabel("KEY", to: key, size: 220)
            // Objectives stay above translucent zones, moving objects and labels.
            key?.strokeColor = .white; key?.lineWidth = 45
            key?.childNode(withName: "caption")?.position.y = 420
            addLabel("EXIT", to: exit, size: 260)
            for mechanic in level.mechanics {
                let shape = mechanicNode(mechanic)
                objects[mechanic.id] = shape
                addChild(shape)
                if mechanic.kind == .fallingRock {
                    let landing = GameRect(x: mechanic.rect.x, y: mechanic.rect.y + mechanic.travel, w: mechanic.rect.w, h: mechanic.rect.h)
                    let warning = node(landing, color: .systemRed.withAlphaComponent(0.25))
                    warning.zPosition = 5; addLabel("⚠ ROCK", to: warning, size: 220)
                    rockWarnings[mechanic.id] = warning; addChild(warning)
                }
            }
        }
        localPlayer = localID
        if lastDeaths != world.deaths { motion.reset() }
        lastDeaths = world.deaths; roster = world.players.map(\.id).sorted()
        motion.append(time: Double(world.tick) / 60, received: ProcessInfo.processInfo.systemUptime,
                      bodies: world.players.map { MotionBody(id: $0.id, position: MotionPoint(x: Double($0.x), y: Double($0.y)),
                                                            velocity: MotionPoint(x: Double($0.vx) * 60, y: Double($0.vy) * 60)) })
        gate?.isHidden = world.key_collected
        key?.isHidden = world.key_collected
        exit?.fillColor = world.key_collected ? .systemGreen : .darkGray
        for object in world.objects {
            guard let shape = objects[object.id], let mechanic = level.mechanics.first(where: { $0.id == object.id }) else { continue }
            shape.position = position(object.rect)
            rockWarnings[object.id]?.isHidden = object.timer >= 60
            // Inactive gates and crumbled ledges are outlines, never solid-looking floors.
            let ghost = !object.active && [.pressureGate, .timedGate, .crumble].contains(mechanic.kind)
            shape.fillColor = ghost ? .clear : mechanicColor(mechanic.kind).withAlphaComponent(zone(mechanic.kind) ? 0.18 : 1)
            shape.alpha = ghost ? 0.3 : 1
            if mechanic.kind == .plate || mechanic.kind == .teamPad { shape.yScale = object.active ? 0.55 : 1 }
            if mechanic.kind == .checkpoint {
                shape.strokeColor = world.checkpoint == object.rect ? .white : .systemGreen
                shape.lineWidth = world.checkpoint == object.rect ? 60 : 25
            }
            if let label = shape.childNode(withName: "caption") as? SKLabelNode {
                switch mechanic.kind {
                case .pressureGate: label.text = object.active ? "PLATE LOCK" : "OPEN"
                case .timedGate: label.text = "\(object.active ? "WAIT" : "GO") \(object.timer / 60)s"
                case .crumble: label.text = object.active ? (object.timer > 0 ? "CRACK!" : "CRUMBLE") : "REFORMING"
                case .plate: label.text = object.active ? "PRESSED" : "STAND"
                case .teamPad: label.text = object.active ? "ENGAGED" : "\(padThreshold(mechanic, fallback: level.minimumPlayers)) TO PRESS"
                case .rope: label.text = object.active ? "↑ HOLD JUMP" : "\(level.minimumPlayers) ON ROPE"
                case .fallingRock: label.text = object.active ? "↓ ROCK" : object.timer < 60 ? "⚠ ROCK" : "RESET"
                default: break
                }
            }
        }
        let liveIDs = Set(world.players.map(\.id))
        for id in Array(players.keys) where !liveIDs.contains(id) {
            entered.remove(id)
            players.removeValue(forKey: id)?.removeFromParent()
            markers.removeValue(forKey: id)?.removeFromParent()
        }
        for player in world.players {
            let rect = GameRect(x: player.x, y: player.y, w: Wire.playerWidth, h: Wire.playerHeight)
            if players[player.id] == nil {
                let sprite = node(rect, color: palette[Int((player.id - 1) % UInt32(palette.count))])
                sprite.zPosition = 10
                let label = SKLabelNode(fontNamed: "AvenirNext-Bold")
                label.text = "P\(player.id)"
                label.fontSize = 280
                label.position = CGPoint(x: 0, y: 640)
                sprite.addChild(label)
                players[player.id] = sprite
                addChild(sprite)
            }
            players[player.id]?.lineWidth = player.id == localID ? 50 : 0
            players[player.id]?.strokeColor = .white
            let atExit = world.key_collected && player.x < level.exit.x + level.exit.w && player.x + Wire.playerWidth > level.exit.x &&
                player.y < level.exit.y + level.exit.h && player.y + Wire.playerHeight > level.exit.y
            if atExit, entered.insert(player.id).inserted, let sprite = players[player.id] {
                sprite.removeAction(forKey: "snapshot")
                if UIAccessibility.isReduceMotionEnabled {
                    sprite.alpha = 0
                } else {
                    sprite.run(.group([.move(to: position(level.exit), duration: 0.35), .scale(to: 0.1, duration: 0.35), .fadeOut(withDuration: 0.35)]), withKey: "door-entry")
                }
            } else if atExit {
                players[player.id]?.removeAction(forKey: "snapshot")
            } else if entered.remove(player.id) != nil, let sprite = players[player.id] {
                // Team respawn or restart must bring an already-entered player back.
                sprite.removeAction(forKey: "door-entry"); sprite.alpha = 1; sprite.setScale(1)
            }
        }
        if let caption = exit?.childNode(withName: "caption") as? SKLabelNode {
            caption.text = entered.isEmpty ? "EXIT" : "\(entered.count)/\(world.players.count) ✓"
        }
        showChat(chat)
        updateCamera()
    }

    /// Attach a short-lived quick-chat bubble above the speaker. Each entry is
    /// shown once, keyed by its `at` timestamp, so repeated snapshots do not
    /// re-add it. A missing sprite (player already gone) is skipped silently.
    private func showChat(_ chat: ChatEntry?) {
        guard let chat, chat.at != lastChatAt else { return }
        lastChatAt = chat.at
        guard let sprite = players[chat.player] else { return }
        sprite.childNode(withName: "chat-bubble")?.removeFromParent()
        let bubble = SKLabelNode(fontNamed: "AvenirNext-Bold")
        bubble.name = "chat-bubble"
        bubble.text = chat.text
        bubble.fontSize = 240
        bubble.fontColor = .white
        bubble.verticalAlignmentMode = .center
        bubble.zPosition = 12
        bubble.position = CGPoint(x: 0, y: 1100)
        sprite.addChild(bubble)
        if UIAccessibility.isReduceMotionEnabled {
            bubble.run(.sequence([.wait(forDuration: 3), .removeFromParent()]))
        } else {
            bubble.alpha = 0
            bubble.run(.fadeIn(withDuration: 0.2), withKey: "chat-fade")
            bubble.run(.sequence([.wait(forDuration: 3), .removeFromParent()]))
        }
    }

    override func update(_ currentTime: TimeInterval) {
        guard let level = currentLevel else { return }
        let now = ProcessInfo.processInfo.systemUptime
        for (id, sprite) in players where !entered.contains(id) {
            if let point = motion.position(id: id, now: now, local: id == localPlayer) {
                sprite.position = CGPoint(x: min(Double(level.width - Wire.playerWidth), max(0, point.x)) + Double(Wire.playerWidth) / 2,
                                          y: Double(level.height) - min(Double(level.height - Wire.playerHeight), max(0, point.y)) - Double(Wire.playerHeight) / 2)
            }
        }
        if tether.parent == nil { tether.zPosition = 4; tether.strokeColor = .systemYellow; tether.lineWidth = 35; addChild(tether) }
        let path = CGMutablePath()
        if level.tetherLength > 0, roster.count > 1 {
            for index in 1..<roster.count {
                let a = roster[index - 1], b = roster[index]
                if entered.contains(a) || entered.contains(b) { continue }
                if let first = players[a], let second = players[b] {
                    path.move(to: first.position); path.addLine(to: second.position)
                }
            }
        }
        tether.path = path
        updateCamera()
    }

    private func updateCamera() {
        guard let level = currentLevel,
              let target = localPlayer.flatMap({ players[$0] }) ?? players.values.first,
              let frame = CameraFraming.calculate(worldWidth: Double(level.width), worldHeight: Double(level.height),
                viewWidth: size.width, viewHeight: size.height, targetX: target.position.x, targetY: target.position.y) else { return }
        followCamera.setScale(frame.scale)
        followCamera.position = CGPoint(x: frame.x, y: frame.y)
        // Elevated objectives can be above the landscape viewport even when
        // their geometry is rendered correctly. Keep their direction visible.
        if keyMarker.parent == nil { followCamera.addChild(keyMarker) }
        if let key, !key.isHidden, let placement = frame.marker(targetX: key.position.x, targetY: key.position.y) {
            keyMarker.isHidden = !placement.offscreen
            keyMarker.position = CGPoint(x: placement.x, y: placement.y)
            keyMarker.text = "\(placement.direction) KEY"
        } else { keyMarker.isHidden = true }
        for (id, player) in players {
            guard let placement = frame.marker(targetX: player.position.x, targetY: player.position.y) else { continue }
            if markers[id] == nil {
                let label = SKLabelNode(fontNamed: "AvenirNext-Bold")
                label.fontColor = .white; label.zPosition = 10_000
                label.verticalAlignmentMode = .center
                followCamera.addChild(label); markers[id] = label
            }
            let marker = markers[id]!
            marker.isHidden = !placement.offscreen || id == localPlayer || entered.contains(id)
            marker.fontSize = 12
            marker.position = CGPoint(x: placement.x, y: placement.y)
            marker.text = "\(placement.direction) P\(id)"
        }
    }

    private func position(_ rect: GameRect) -> CGPoint {
        CGPoint(x: Double(rect.x) + Double(rect.w) / 2, y: Double(currentLevel?.height ?? 14336) - Double(rect.y) - Double(rect.h) / 2)
    }
    private func node(_ rect: GameRect, color: UIColor) -> SKShapeNode {
        let result = SKShapeNode(rectOf: CGSize(width: rect.w, height: rect.h), cornerRadius: 75)
        result.position = position(rect)
        result.fillColor = color
        result.lineWidth = 0
        return result
    }

    private func addLabel(_ text: String, to shape: SKShapeNode?, size: CGFloat) {
        let label = SKLabelNode(fontNamed: "AvenirNext-Bold")
        label.name = "caption"
        label.text = text
        label.fontSize = size
        label.verticalAlignmentMode = .center
        label.fontColor = .white
        label.zPosition = 2
        shape?.addChild(label)
    }

    private func zone(_ kind: MechanicKind) -> Bool { [.wind, .lowGravity, .checkpoint, .rope].contains(kind) }

    /// A team pad's head count comes from its own `travel` threshold, clamped
    /// to 2...4 exactly like the simulation, falling back to the level's team
    /// size when `travel` carries no usable count.
    private func padThreshold(_ mechanic: GameMechanic, fallback: Int) -> Int {
        let needed = mechanic.travel > 1 ? mechanic.travel : fallback
        return max(2, min(4, needed))
    }

    private func mechanicColor(_ kind: MechanicKind) -> UIColor {
        switch kind {
        case .hazard: .systemRed
        case .plate, .pressureGate: .systemOrange
        case .teamPad: .systemTeal
        case .timedGate: .systemPink
        case .oneWay, .movingPlatform: .systemTeal
        case .conveyor: .systemYellow
        case .ice: .systemCyan
        case .spring: .systemGreen
        case .checkpoint: .systemGreen
        case .crumble: .systemBrown
        case .lowGravity: .systemPurple
        case .wind: .systemBlue
        case .rope: .systemBrown
        case .fallingRock: .systemGray
        }
    }

    private func mechanicNode(_ mechanic: GameMechanic) -> SKShapeNode {
        let shape = node(mechanic.rect, color: mechanicColor(mechanic.kind))
        shape.zPosition = zone(mechanic.kind) ? -1 : 1
        shape.strokeColor = mechanicColor(mechanic.kind)
        shape.lineWidth = 25
        let w = CGFloat(mechanic.rect.w), h = CGFloat(mechanic.rect.h)
        let path = CGMutablePath()
        // Different silhouettes/patterns communicate behavior without relying on color.
        switch mechanic.kind {
        case .rope:
            path.move(to: CGPoint(x: 0, y: -h / 2)); path.addLine(to: CGPoint(x: 0, y: h / 2))
            for i in 0..<min(64, mechanic.rect.h / 512) {
                let y = -h / 2 + CGFloat(i) * 512
                path.move(to: CGPoint(x: -80, y: y)); path.addLine(to: CGPoint(x: 80, y: y))
            }
        case .hazard:
            path.move(to: CGPoint(x: -w / 2, y: -h / 2))
            let teeth = max(1, min(24, mechanic.rect.w / 350))
            for i in 0..<teeth {
                path.addLine(to: CGPoint(x: -w / 2 + w * (CGFloat(i) + 0.5) / CGFloat(teeth), y: h / 2))
                path.addLine(to: CGPoint(x: -w / 2 + w * CGFloat(i + 1) / CGFloat(teeth), y: -h / 2))
            }
            path.closeSubpath(); shape.path = path
        case .oneWay:
            path.move(to: CGPoint(x: -w / 2, y: h / 2)); path.addLine(to: CGPoint(x: w / 2, y: h / 2))
        case .pressureGate, .timedGate:
            for fraction in [-0.3, 0.0, 0.3] {
                let x = w * fraction
                path.move(to: CGPoint(x: x, y: -h / 2)); path.addLine(to: CGPoint(x: x, y: h / 2))
            }
        case .ice:
            for fraction in [-0.3, 0.0, 0.3] {
                let x = w * fraction
                path.move(to: CGPoint(x: x - 90, y: -h / 2)); path.addLine(to: CGPoint(x: x + 90, y: h / 2))
            }
        case .spring:
            path.move(to: CGPoint(x: -w / 3, y: -h / 2))
            for i in 1...6 { path.addLine(to: CGPoint(x: i.isMultiple(of: 2) ? -w / 3 : w / 3, y: -h / 2 + h * CGFloat(i) / 6)) }
        case .crumble:
            path.move(to: CGPoint(x: -w / 4, y: h / 2)); path.addLine(to: CGPoint(x: 0, y: 0))
            path.addLine(to: CGPoint(x: -w / 8, y: -h / 4)); path.addLine(to: CGPoint(x: w / 4, y: -h / 2))
        default: break
        }
        if mechanic.kind != .hazard && !path.isEmpty {
            let pattern = SKShapeNode(path: path)
            pattern.strokeColor = .white; pattern.lineWidth = 45
            shape.addChild(pattern)
        }
        let caption: String
        switch mechanic.kind {
        case .plate: caption = "STAND"
        case .teamPad: caption = "\(padThreshold(mechanic, fallback: currentLevel?.minimumPlayers ?? 2)) TO PRESS"
        case .pressureGate: caption = "PLATE LOCK"
        case .timedGate: caption = "WAIT"
        case .oneWay: caption = "↑ THROUGH"
        case .movingPlatform: caption = "↔ RIDE"
        case .conveyor: caption = mechanic.travel < 0 ? "≪ BELT" : "BELT ≫"
        case .ice: caption = "ICE ~"
        case .spring: caption = "↑↑"
        case .hazard: caption = "!"
        case .checkpoint: caption = "⚑ SAVE"
        case .crumble: caption = "CRUMBLE"
        case .lowGravity: caption = "↑ LOW G"
        case .wind: caption = mechanic.travel < 0 ? "← WIND" : "WIND →"
        case .rope: caption = "↑ HOLD JUMP TOGETHER"
        case .fallingRock: caption = "⚠ ROCK"
        }
        addLabel(caption, to: shape, size: min(260, max(140, w / CGFloat(max(1, caption.count)))))
        if h < 500 { shape.childNode(withName: "caption")?.position.y = h / 2 + 180 }
        return shape
    }
}

extension MechanicKind {
    var legend: String {
        switch self {
        case .plate: "▁ Stand on plates"
        case .teamPad: "◎ Team pad: stand together to press"
        case .pressureGate: "▥ Plates open gates"
        case .timedGate: "◷ Wait for gates"
        case .oneWay: "↑ Jump through ledges"
        case .movingPlatform: "↔ Ride platforms"
        case .conveyor: "≫ Moving belts"
        case .ice: "~ Slippery ice"
        case .spring: "↑↑ Spring boost"
        case .hazard: "▲ Hazard: team respawns"
        case .checkpoint: "⚑ Team checkpoint"
        case .crumble: "⋯ Ledges crumble, then reform"
        case .lowGravity: "↑ Low gravity zone"
        case .wind: "→ Wind pushes players"
        case .rope: "↟ Climb together: hold Jump on the rope"
        case .fallingRock: "↓ Falling rocks: watch warnings, cross together"
        }
    }
}

struct GameSurface: UIViewRepresentable {
    let level: GameLevel?
    let world: GameWorld?
    let localID: UInt32?
    var chat: ChatEntry?
    func makeUIView(context: Context) -> SKView {
        let view = SKView()
        view.preferredFramesPerSecond = 60
        view.ignoresSiblingOrder = false
        view.isMultipleTouchEnabled = true
        view.presentScene(GameScene(size: CGSize(width: 1, height: 1)))
        return view
    }
    func updateUIView(_ view: SKView, context: Context) {
        (view.scene as? GameScene)?.present(level: level, world: world, localID: localID, chat: chat)
    }
    static func dismantleUIView(_ view: SKView, coordinator: ()) { view.presentScene(nil) }
}
