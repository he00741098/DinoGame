import SpriteKit
import SwiftUI

/// GPU-backed presentation only. No physics or authority is implemented in SpriteKit.
@MainActor
final class GameScene: SKScene {
    private var currentLevel: GameLevel?
    private var players: [UInt32: SKShapeNode] = [:]
    private var objects: [UInt32: SKShapeNode] = [:]
    private var gate: SKShapeNode?
    private var key: SKShapeNode?
    private var exit: SKShapeNode?
    private let palette: [UIColor] = [.systemMint, .systemCyan, .systemOrange, .systemPurple]

    override init(size: CGSize) {
        super.init(size: size)
        scaleMode = .aspectFit
        backgroundColor = UIColor(red: 0.06, green: 0.10, blue: 0.17, alpha: 1)
    }
    required init?(coder: NSCoder) { fatalError("Programmatic scene only") }

    func present(level: GameLevel?, world: GameWorld?, localID: UInt32?) {
        guard let level, let world else {
            removeAllChildren(); players.removeAll(); objects.removeAll(); currentLevel = nil
            gate = nil; key = nil; exit = nil
            return
        }
        if currentLevel != level {
            removeAllChildren(); players.removeAll(); objects.removeAll()
            currentLevel = level
            size = CGSize(width: level.width, height: level.height)
            for rect in level.solids { addChild(node(rect, color: UIColor(red: 0.24, green: 0.34, blue: 0.44, alpha: 1))) }
            gate = node(level.gate, color: .systemIndigo)
            key = node(level.key, color: .systemYellow)
            exit = node(level.exit, color: .darkGray)
            for item in [gate, key, exit].compactMap({ $0 }) { addChild(item) }
            addLabel("KEY", to: key, size: 220)
            addLabel("EXIT", to: exit, size: 260)
            for mechanic in level.mechanics {
                let shape = mechanicNode(mechanic)
                objects[mechanic.id] = shape
                addChild(shape)
            }
        }
        gate?.isHidden = world.key_collected
        key?.isHidden = world.key_collected
        exit?.fillColor = world.key_collected ? .systemGreen : .darkGray
        for object in world.objects {
            guard let shape = objects[object.id], let mechanic = level.mechanics.first(where: { $0.id == object.id }) else { continue }
            shape.position = position(object.rect)
            // Inactive gates and crumbled ledges are outlines, never solid-looking floors.
            let ghost = !object.active && [.pressureGate, .timedGate, .crumble].contains(mechanic.kind)
            shape.fillColor = ghost ? .clear : mechanicColor(mechanic.kind).withAlphaComponent(zone(mechanic.kind) ? 0.18 : 1)
            shape.alpha = ghost ? 0.3 : 1
            if mechanic.kind == .plate { shape.yScale = object.active ? 0.55 : 1 }
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
                default: break
                }
            }
        }
        let liveIDs = Set(world.players.map(\.id))
        for id in Array(players.keys) where !liveIDs.contains(id) {
            players.removeValue(forKey: id)?.removeFromParent()
        }
        for player in world.players {
            let rect = GameRect(x: player.x, y: player.y, w: Wire.playerWidth, h: Wire.playerHeight)
            if let existing = players[player.id] {
                existing.removeAction(forKey: "snapshot")
                // Presentation smoothing only; snapshots and collision coordinates stay untouched.
                existing.run(.move(to: position(rect), duration: 0.05), withKey: "snapshot")
            } else {
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
        }
    }

    private func position(_ rect: GameRect) -> CGPoint {
        CGPoint(x: Double(rect.x) + Double(rect.w) / 2, y: size.height - Double(rect.y) - Double(rect.h) / 2)
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

    private func zone(_ kind: MechanicKind) -> Bool { [.wind, .lowGravity, .checkpoint].contains(kind) }

    private func mechanicColor(_ kind: MechanicKind) -> UIColor {
        switch kind {
        case .hazard: .systemRed
        case .plate, .pressureGate: .systemOrange
        case .timedGate: .systemPink
        case .oneWay, .movingPlatform: .systemTeal
        case .conveyor: .systemYellow
        case .ice: .systemCyan
        case .spring: .systemGreen
        case .checkpoint: .systemGreen
        case .crumble: .systemBrown
        case .lowGravity: .systemPurple
        case .wind: .systemBlue
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
        }
    }
}

struct GameSurface: UIViewRepresentable {
    let level: GameLevel?
    let world: GameWorld?
    let localID: UInt32?
    func makeUIView(context: Context) -> SKView {
        let view = SKView()
        view.preferredFramesPerSecond = 60
        view.ignoresSiblingOrder = false
        view.isMultipleTouchEnabled = true
        view.presentScene(GameScene(size: CGSize(width: 24576, height: 14336)))
        return view
    }
    func updateUIView(_ view: SKView, context: Context) {
        (view.scene as? GameScene)?.present(level: level, world: world, localID: localID)
    }
    static func dismantleUIView(_ view: SKView, coordinator: ()) { view.presentScene(nil) }
}
