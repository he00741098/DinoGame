import XCTest

final class CoopClientUITests: XCTestCase {
    @MainActor
    func testInvalidEndpointFailsWithoutConnecting() {
        let app = XCUIApplication()
        app.launchEnvironment["COOP_ENDPOINT"] = "ws://public.example.com"
        app.launch()
        app.buttons["join"].tap()
        XCTAssertTrue(app.staticTexts["Check connection settings"].waitForExistence(timeout: 3))
        XCTAssertFalse(app.buttons["moveLeft"].isEnabled)
    }

    /// Requires the development server on host loopback:3000. No mock server or
    /// client-side positions: this exercises Foundation WebSockets on iOS.
    @MainActor
    func testNativeAuthorityMovementAndBackgroundResume() throws {
        let app = XCUIApplication()
        app.launchEnvironment["COOP_ENDPOINT"] = ProcessInfo.processInfo.environment["COOP_TEST_ENDPOINT"] ?? "ws://127.0.0.1:3000"
        app.launchEnvironment["COOP_ROOM"] = ProcessInfo.processInfo.environment["COOP_TEST_ROOM"] ?? "playtest"
        app.launch()
        app.buttons["join"].tap()
        let connected = NSPredicate(format: "label BEGINSWITH %@", "Connected · P")
        let status = app.staticTexts["connectionStatus"]
        expectation(for: connected, evaluatedWith: status)
        waitForExpectations(timeout: 15)
        let position = app.staticTexts["playerPosition"]
        XCTAssertTrue(position.waitForExistence(timeout: 3))
        if ProcessInfo.processInfo.environment["COOP_EXPECT_PEER"] == "1" {
            let peers = NSPredicate(format: "label BEGINSWITH %@", "2/4 players")
            expectation(for: peers, evaluatedWith: app.staticTexts["diagnostics"])
            waitForExpectations(timeout: 5)
        }
        let before = position.label
        app.buttons["moveRight"].press(forDuration: 0.6)
        let moved = NSPredicate { _, _ in position.label != before }
        expectation(for: moved, evaluatedWith: nil)
        waitForExpectations(timeout: 4)
        let diagnostics = app.staticTexts["diagnostics"]
        let risesBefore = UInt64(diagnostics.label.components(separatedBy: "rises ").last ?? "") ?? 0
        app.buttons["jump"].tap()
        let jumped = NSPredicate { _, _ in
            let rises = UInt64(diagnostics.label.components(separatedBy: "rises ").last ?? "") ?? 0
            return rises > risesBefore
        }
        expectation(for: jumped, evaluatedWith: nil)
        waitForExpectations(timeout: 3)
        let capture = XCTAttachment(screenshot: app.screenshot())
        capture.name = "Native cross-play"
        capture.lifetime = .keepAlways
        add(capture)
        XCUIDevice.shared.orientation = .landscapeLeft
        defer { XCUIDevice.shared.orientation = .portrait }
        let wide = NSPredicate { _, _ in
            let frame = app.windows.firstMatch.frame
            return frame.width > frame.height
        }
        expectation(for: wide, evaluatedWith: nil)
        waitForExpectations(timeout: 5)
        XCTAssertTrue(app.buttons["moveRight"].isHittable)
        XCTAssertTrue(app.buttons["jump"].isHittable)
        let landscape = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        landscape.name = "Native landscape"
        landscape.lifetime = .keepAlways
        add(landscape)
        XCUIDevice.shared.orientation = .portrait
        let tall = NSPredicate { _, _ in
            let frame = app.windows.firstMatch.frame
            return frame.height > frame.width
        }
        expectation(for: tall, evaluatedWith: nil)
        waitForExpectations(timeout: 5)
        let identity = String(status.label.split(separator: "P").last ?? "")
        XCUIDevice.shared.press(.home)
        app.activate()
        expectation(for: connected, evaluatedWith: status)
        waitForExpectations(timeout: 15)
        XCTAssertEqual(String(status.label.split(separator: "P").last ?? ""), identity)
        app.buttons["leave"].tap()
        XCTAssertTrue(app.staticTexts["Not connected"].waitForExistence(timeout: 3))
        XCTAssertFalse(app.buttons["moveLeft"].isEnabled)
    }

    @MainActor
    func testFailedPrimaryFallsBackThroughSameSessionProtocol() {
        let app = XCUIApplication()
        app.launchEnvironment["COOP_ENDPOINT"] = "ws://127.0.0.1:1"
        app.launchEnvironment["COOP_FALLBACK"] = ProcessInfo.processInfo.environment["COOP_TEST_ENDPOINT"] ?? "ws://127.0.0.1:3000"
        app.launchEnvironment["COOP_ROOM"] = "ios-fallback"
        app.launch()
        app.buttons["join"].tap()
        let connected = NSPredicate(format: "label BEGINSWITH %@", "Connected · P")
        expectation(for: connected, evaluatedWith: app.staticTexts["connectionStatus"])
        waitForExpectations(timeout: 15)
        XCTAssertTrue(app.buttons["moveRight"].isEnabled)
        app.buttons["leave"].tap()
    }

    @MainActor
    func testControllerRestartPreservesIdentityAndReactivatesControls() {
        let app = XCUIApplication()
        app.launchEnvironment["COOP_ENDPOINT"] = ProcessInfo.processInfo.environment["COOP_TEST_ENDPOINT"] ?? "ws://127.0.0.1:3000"
        app.launchEnvironment["COOP_ROOM"] = "ios-restart"
        app.launch()
        app.buttons["join"].tap()
        let status = app.staticTexts["connectionStatus"]
        let connected = NSPredicate(format: "label BEGINSWITH %@", "Connected · P")
        expectation(for: connected, evaluatedWith: status)
        waitForExpectations(timeout: 15)
        let identity = status.label
        let originalPosition = app.staticTexts["playerPosition"].label
        let title = app.staticTexts["levelTitle"]
        let round = title.value as? String
        app.buttons["moveRight"].press(forDuration: 0.3)
        app.buttons["restart"].tap()
        let restarted = NSPredicate { _, _ in (title.value as? String) != round }
        expectation(for: restarted, evaluatedWith: nil)
        waitForExpectations(timeout: 5)
        expectation(for: connected, evaluatedWith: status)
        waitForExpectations(timeout: 5)
        XCTAssertEqual(status.label, identity)
        XCTAssertEqual(app.staticTexts["playerPosition"].label, originalPosition)
        XCTAssertTrue(app.buttons["moveRight"].isEnabled)
        app.buttons["leave"].tap()
    }

    /// Requires the development server on host loopback:3000, like the other
    /// live-session tests. The harness never echoes chat back, so the chatLog
    /// caption is allowed to stay empty after sending.
    @MainActor
    func testQuickChatButtonsExistAndAreGatedOnActiveSession() {
        let app = XCUIApplication()
        app.launchEnvironment["COOP_ENDPOINT"] = ProcessInfo.processInfo.environment["COOP_TEST_ENDPOINT"] ?? "ws://127.0.0.1:3000"
        app.launchEnvironment["COOP_ROOM"] = "ios-quickchat"
        app.launch()
        for index in 0..<6 {
            let button = app.buttons["chat\(index)"]
            XCTAssertTrue(button.waitForExistence(timeout: 3), "chat\(index) missing before join")
            XCTAssertFalse(button.isEnabled, "chat\(index) must be disabled without an active session")
        }
        app.buttons["join"].tap()
        let status = app.staticTexts["connectionStatus"]
        let connected = NSPredicate(format: "label BEGINSWITH %@", "Connected · P")
        expectation(for: connected, evaluatedWith: status)
        waitForExpectations(timeout: 15)
        let chat2 = app.buttons["chat2"]
        let enabled = NSPredicate(format: "isEnabled == true")
        expectation(for: enabled, evaluatedWith: chat2)
        waitForExpectations(timeout: 5)
        XCTAssertTrue(chat2.isHittable)
        chat2.tap()
        // No crash and no disconnect after sending; the caption itself is not
        // asserted because the harness does not echo chat.
        XCTAssertEqual(app.state, .runningForeground)
        expectation(for: connected, evaluatedWith: status)
        waitForExpectations(timeout: 5)
        // Sending starts the two-second cooldown that matches the server's own
        // rate limit, so the button is deliberately disabled right now; it must
        // come back on its own rather than staying stuck.
        let reenabled = NSPredicate(format: "isEnabled == true")
        expectation(for: reenabled, evaluatedWith: chat2)
        waitForExpectations(timeout: 6)
        XCTAssertTrue(chat2.isEnabled, "the chat cooldown must release on its own")
        app.buttons["leave"].tap()
    }

    @MainActor
    func testFreshSnapshotsWithoutInputAcksFailOverInsteadOfFreezing() throws {
        let proxy = try XCTUnwrap(ProcessInfo.processInfo.environment["COOP_ACK_DROP_ENDPOINT"],
                                 "Run via native/scripts/test_ios.py for real fault injection")
        let app = XCUIApplication()
        app.launchEnvironment["COOP_ENDPOINT"] = proxy
        app.launchEnvironment["COOP_FALLBACK"] = ProcessInfo.processInfo.environment["COOP_TEST_ENDPOINT"] ?? "ws://127.0.0.1:3000"
        app.launchEnvironment["COOP_ROOM"] = "ios-ack-stall"
        app.launch()
        app.buttons["join"].tap()
        let connected = NSPredicate(format: "label BEGINSWITH %@", "Connected · P")
        expectation(for: connected, evaluatedWith: app.staticTexts["connectionStatus"])
        waitForExpectations(timeout: 15)
        XCTAssertTrue(app.buttons["moveRight"].isEnabled)
        app.buttons["leave"].tap()
    }
}
