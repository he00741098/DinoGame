import Testing
@testable import MultiplayerKit

@Test func portraitCameraKeepsCharactersReadableOnLongStagesAndGroundVisible() throws {
    let candidate = CameraFraming.calculate(worldWidth: 40960, worldHeight: 14336,
        viewWidth: 375, viewHeight: 500, targetX: 3072, targetY: 2560)
    let frame = try #require(candidate)
    #expect(abs(frame.visibleWidth - 14336) < 0.000001)
    #expect(Double(Wire.playerWidth) / frame.scale >= 18)
    #expect(frame.y - frame.visibleHeight / 2 == 0)
    #expect(frame.x - frame.visibleWidth / 2 == 0)
}

@Test func cameraTracksFarRightWithoutChangingWorldBounds() throws {
    let candidate = CameraFraming.calculate(worldWidth: 40960, worldHeight: 14336,
        viewWidth: 375, viewHeight: 500, targetX: 39936, targetY: 2048)
    let frame = try #require(candidate)
    #expect(frame.x + frame.visibleWidth / 2 == 40 * 1024)
}

@Test func landscapeCameraRetainsJumpHeadroomAndHighKeys() throws {
    let candidate = CameraFraming.calculate(worldWidth: 40960, worldHeight: 14336,
        viewWidth: 800, viewHeight: 180, targetX: 20480, targetY: 11264)
    let frame = try #require(candidate)
    #expect(frame.visibleHeight >= 6 * 1024)
    #expect(frame.y + frame.visibleHeight / 2 <= 14 * 1024)
    #expect(frame.y - frame.visibleHeight / 2 < 11 * 1024)
}

@Test func cameraRejectsUnlaidOutOrNonfiniteViews() {
    let unlaid = CameraFraming.calculate(worldWidth: 10, worldHeight: 10, viewWidth: 0, viewHeight: 100, targetX: 0, targetY: 0)
    let nonfinite = CameraFraming.calculate(worldWidth: .infinity, worldHeight: 10, viewWidth: 100, viewHeight: 100, targetX: 0, targetY: 0)
    let tiny = CameraFraming.calculate(worldWidth: 24576, worldHeight: 14336, viewWidth: 1, viewHeight: 1, targetX: 0, targetY: 0)
    #expect(unlaid == nil)
    #expect(nonfinite == nil)
    #expect(tiny == nil)
}

@Test func teammateMarkersStayInsideScreenCoordinatesAtHighWorldZoom() throws {
    let candidate = CameraFraming.calculate(worldWidth: 40960, worldHeight: 14336,
        viewWidth: 375, viewHeight: 500, targetX: 3072, targetY: 2560)
    let frame = try #require(candidate)
    let placement = frame.marker(targetX: 39936, targetY: 2000)
    let marker = try #require(placement)
    #expect(marker.offscreen)
    #expect(marker.direction == "→")
    #expect(abs(marker.x) <= 375.0 / 2 - 18)
    #expect(abs(marker.y) <= 500.0 / 2 - 18)
}
