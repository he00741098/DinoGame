import Foundation
import Testing
@testable import MultiplayerKit

@Test func playtestCredentialIsHeaderOnlyAndNeverSentToFallback() throws {
    let primary = try ConnectionRoute("wss://primary.example", allowLocalInsecure: false)
    let fallback = try ConnectionRoute("wss://fallback.example", allowLocalInsecure: false)
    let header = try #require(PlaytestAccess.header(code: "test-code", route: primary, primary: primary))
    #expect(header == "Basic " + Data("playtester:test-code".utf8).base64EncodedString())
    #expect(PlaytestAccess.header(code: "test-code", route: fallback, primary: primary) == nil)
    #expect(PlaytestAccess.header(code: "", route: primary, primary: primary) == nil)
    #expect(primary.url.user == nil && primary.url.password == nil)
}
