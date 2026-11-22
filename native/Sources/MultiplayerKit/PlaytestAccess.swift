import Foundation

/// Optional HTTPS playtest gate. Never put credentials in a URL, logs, or
/// preferences, and never forward them to an unrelated fallback endpoint.
public enum PlaytestAccess {
    public static func header(code: String, route: ConnectionRoute, primary: ConnectionRoute) -> String? {
        guard route == primary, !code.isEmpty, code.utf8.count <= 256 else { return nil }
        return "Basic " + Data("playtester:\(code)".utf8).base64EncodedString()
    }
}
