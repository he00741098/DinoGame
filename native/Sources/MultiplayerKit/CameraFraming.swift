import Foundation

/// Presentation-only camera math. Viewport size never changes simulation coordinates.
public struct CameraFraming: Equatable, Sendable {
    public let x: Double
    public let y: Double
    public let scale: Double
    public let visibleWidth: Double
    public let visibleHeight: Double

    public struct Marker: Equatable, Sendable {
        public let x: Double
        public let y: Double
        public let offscreen: Bool
        public let direction: String
    }

    /// Camera children are screen-space: undo world-units-per-point before
    /// clamping an indicator, rather than positioning a HUD in world units.
    public func marker(targetX: Double, targetY: Double) -> Marker? {
        guard targetX.isFinite, targetY.isFinite else { return nil }
        let dx = (targetX - x) / scale, dy = (targetY - y) / scale
        let halfW = max(0, visibleWidth / scale / 2 - 18)
        let halfH = max(0, visibleHeight / scale / 2 - 18)
        return Marker(x: min(halfW, max(-halfW, dx)), y: min(halfH, max(-halfH, dy)),
            offscreen: abs(dx) > halfW || abs(dy) > halfH,
            direction: dx < -halfW ? "←" : dx > halfW ? "→" : dy > halfH ? "↑" : "↓")
    }

    public static func calculate(worldWidth: Double, worldHeight: Double, viewWidth: Double,
                                 viewHeight: Double, targetX: Double, targetY: Double) -> Self? {
        guard [worldWidth, worldHeight, viewWidth, viewHeight, targetX, targetY].allSatisfy(\.isFinite),
              (1...1_000_000).contains(worldWidth), (1...1_000_000).contains(worldHeight),
              (44...16_384).contains(viewWidth), (44...16_384).contains(viewHeight) else { return nil }
        // Readable characters on long stages; preserve at least six tiles vertically
        // on landscape screens rather than cropping the useful jump space.
        let scale = max(14 * 1024 / viewWidth, 6 * 1024 / viewHeight)
        let width = viewWidth * scale, height = viewHeight * scale
        let x = width >= worldWidth ? worldWidth / 2 : min(worldWidth - width / 2, max(width / 2, targetX))
        let y = height >= worldHeight ? height / 2 : min(worldHeight - height / 2, max(height / 2, targetY + 1024))
        return Self(x: x, y: y, scale: scale, visibleWidth: width, visibleHeight: height)
    }
}
