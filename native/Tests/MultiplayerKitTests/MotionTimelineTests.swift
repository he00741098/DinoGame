import Testing
@testable import MultiplayerKit

private func body(_ x: Double, vx: Double = 0) -> [MotionBody] {
    [MotionBody(id: 1, position: MotionPoint(x: x, y: 0), velocity: MotionPoint(x: vx, y: 0))]
}

@Test func bufferedMotionInterpolatesAndNeverMovesBackwards() {
    var timeline = MotionTimeline()
    timeline.append(time: 0, received: 10, bodies: body(0))
    timeline.append(time: 0.05, received: 10.05, bodies: body(1000))
    var previous = -1.0
    for step in 0..<10 {
        let point = timeline.position(id: 1, now: 10.0 + Double(step) * 0.016, local: true)
        #expect(point != nil && point!.x >= previous)
        previous = point!.x
    }
}

@Test func stalledPacketsExtrapolateBrieflyWithinAHardCap() {
    var timeline = MotionTimeline()
    timeline.append(time: 0, received: 10, bodies: body(0, vx: 9600))
    timeline.append(time: 0.05, received: 10.05, bodies: body(480, vx: 9600))
    var last = 0.0
    var now = 10.05
    while now < 11 { last = timeline.position(id: 1, now: now, local: true)!.x; now += 0.016 }
    #expect(last > 480 && last <= 480 + 1536 + 1)
}

@Test func teleportsAndNonFiniteFramesCannotCreateInvalidPositions() {
    var timeline = MotionTimeline()
    timeline.append(time: 1, received: 1, bodies: body(0))
    timeline.append(time: 1.05, received: 1.05, bodies: body(100_000)) // respawn/teleport resets
    let jumped = timeline.position(id: 1, now: 1.06, local: true)
    #expect(jumped != nil && jumped!.x >= 99_000)
    var bad = MotionTimeline()
    bad.append(time: .nan, received: 1, bodies: body(0))
    bad.append(time: 1, received: 1, bodies: body(.infinity))
    #expect(bad.position(id: 1, now: 2) == nil)
}
