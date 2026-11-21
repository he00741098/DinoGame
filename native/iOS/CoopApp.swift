import SwiftUI
import UIKit

@main
struct CoopApp: App {
    @StateObject private var session = SessionModel()
    @Environment(\.scenePhase) private var phase
    var body: some Scene {
        WindowGroup {
            LobbyView(session: session)
                .preferredColorScheme(.dark)
                .onChange(of: phase) { _, value in session.setForeground(value == .active) }
        }
    }
}

struct LobbyView: View {
    @ObservedObject var session: SessionModel
    @State private var endpoint = ProcessInfo.processInfo.environment["COOP_ENDPOINT"] ?? "ws://127.0.0.1:3000"
    @State private var fallback = ProcessInfo.processInfo.environment["COOP_FALLBACK"] ?? ""
    @State private var room = ProcessInfo.processInfo.environment["COOP_ROOM"] ?? "playtest"
    @FocusState private var editing: Bool
    @State private var connectionExpanded = false
    @State private var legendExpanded = false

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if session.world == nil {
                ScrollView {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("CO-OP LAB").font(.title2.bold()).foregroundStyle(.mint)
                        Text("20 challenges. One team.").font(.headline)
                        connectionForm
                        Text("2–4 players. Stack on teammates, collect the shared key, then bring everyone to the exit.")
                            .font(.footnote).foregroundStyle(.secondary)
                    }
                }.scrollDismissesKeyboard(.interactively)
            } else {
                campaignHeader
                DisclosureGroup("Room & server", isExpanded: $connectionExpanded) {
                    connectionForm
                }.font(.caption)
            }
            HStack {
                Text(session.status).font(.caption.bold()).accessibilityIdentifier("connectionStatus")
                Spacer()
                if session.requested || session.world != nil {
                    Button("Leave", role: .destructive) { session.leave() }.accessibilityIdentifier("leave")
                        .font(.caption)
                }
            }
            Text("\(session.world?.players.count ?? 0)/4 players · tick \(session.world?.tick ?? 0) · ack \(session.appliedSequence) · rises \(session.upwardTransitions)")
                .font(.caption2.monospacedDigit()).accessibilityIdentifier("diagnostics")
            if !session.active {
                Text(session.detail).font(.caption2).foregroundStyle(.secondary).lineLimit(2)
            }
            if session.world != nil {
                GameSurface(level: session.level, world: session.world, localID: session.playerID)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                    .accessibilityLabel("Cooperative game world")
                    .overlay(alignment: .topLeading) {
                        if !session.active {
                            Text("Controls paused").font(.caption.bold()).padding(8)
                                .background(.black.opacity(0.6), in: RoundedRectangle(cornerRadius: 8))
                        }
                    }
                if let player = session.world?.players.first(where: { $0.id == session.playerID }) {
                    Text("P\(player.id) x=\(player.x) y=\(player.y)")
                        .font(.caption2.monospacedDigit()).foregroundStyle(.secondary)
                        .accessibilityIdentifier("playerPosition")
                }
            }
        }
        .padding(.horizontal, 12).padding(.top, 6)
        .safeAreaInset(edge: .bottom, spacing: 6) { controls.padding(.horizontal, 12).padding(.bottom, 6) }
        .background(Color(red: 0.04, green: 0.07, blue: 0.12))
    }

    private var connectionForm: some View {
        VStack(spacing: 8) {
            TextField("Server endpoint", text: $endpoint)
                .accessibilityIdentifier("endpoint").textContentType(.URL).keyboardType(.URL)
            TextField("Fallback endpoint (optional)", text: $fallback)
                .accessibilityIdentifier("fallback").keyboardType(.URL)
            HStack {
                TextField("Room code", text: $room).accessibilityIdentifier("room")
                Button("Join") {
                    editing = false
                    connectionExpanded = false
                    session.connect(primary: endpoint, fallback: fallback, room: room)
                }.buttonStyle(.borderedProminent).tint(.mint).accessibilityIdentifier("join")
            }
        }
        .textFieldStyle(.roundedBorder).textInputAutocapitalization(.never)
        .autocorrectionDisabled().focused($editing)
    }

    private var campaignHeader: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(session.campaignComplete ? "Campaign complete!" : "\(session.level?.number ?? 1)/20 · \(session.level?.title ?? "Co-op campaign")")
                    .font(.headline).accessibilityIdentifier("levelTitle")
                Spacer(minLength: 4)
                if session.controllerID != nil && session.controllerID == session.playerID {
                    Button("Restart") { session.restartLevel() }
                        .disabled(!session.active).accessibilityIdentifier("restart")
                    if session.world?.completed == true && !session.campaignComplete {
                        Button("Next") { session.nextLevel() }
                            .disabled(!session.active).accessibilityIdentifier("next")
                    }
                }
            }.font(.caption)
            Text(session.campaignComplete ? "All 20 levels cleared together. Thanks for playing!" : (session.level?.hint ?? ""))
                .font(.caption).foregroundStyle(.secondary).lineLimit(2).accessibilityIdentifier("levelHint")
            HStack {
                Text("Team deaths: \(session.world?.deaths ?? 0)")
                Text(session.world?.key_collected == true ? "Key collected ✓" : "Find the key")
                    .foregroundStyle(session.world?.key_collected == true ? .mint : .yellow)
                if session.world?.completed == true && !session.campaignComplete {
                    Text(session.controllerID == session.playerID ? "Level cleared!" : "Cleared — waiting for host")
                }
            }.font(.caption2).accessibilityIdentifier("teamProgress")
            if let mechanics = session.level?.mechanics, !mechanics.isEmpty {
                DisclosureGroup("Mechanic guide", isExpanded: $legendExpanded) {
                    Text(Array(Set(mechanics.map(\.kind))).sorted { $0.rawValue < $1.rawValue }.map(\.legend).joined(separator: " · "))
                        .font(.caption2).foregroundStyle(.secondary)
                }.font(.caption2)
            }
        }
    }

    private var controls: some View {
        HStack(spacing: 12) {
            HoldButton(title: "Left", symbol: "arrow.left", id: "moveLeft") { session.set(.left, source: "left-button", pressed: $0) }
            HoldButton(title: "Right", symbol: "arrow.right", id: "moveRight") { session.set(.right, source: "right-button", pressed: $0) }
            Spacer(minLength: 16)
            HoldButton(title: "Jump", symbol: "arrow.up", id: "jump") { session.set(.jump, source: "jump-button", pressed: $0) }
        }
        .frame(height: 60).disabled(!session.active)
    }
}

/// UIControl tracking handles drag-out/cancel as well as release. A SwiftUI tap
/// alone cannot represent held movement, and a DragGesture can miss cancellation.
struct HoldButton: UIViewRepresentable {
    let title: String
    let symbol: String
    let id: String
    let onPress: (Bool) -> Void
    @Environment(\.isEnabled) private var enabled

    func makeUIView(context: Context) -> HeldControl {
        let button = HeldControl(type: .system)
        var config = UIButton.Configuration.tinted()
        config.title = title
        config.image = UIImage(systemName: symbol)
        config.imagePadding = 8
        config.cornerStyle = .large
        config.baseForegroundColor = .systemMint
        button.configuration = config
        button.accessibilityIdentifier = id
        button.accessibilityLabel = title
        button.onPress = onPress
        return button
    }
    func updateUIView(_ view: HeldControl, context: Context) {
        view.onPress = onPress
        view.isEnabled = enabled
        if !enabled { view.release() }
    }
    static func dismantleUIView(_ view: HeldControl, coordinator: ()) { view.release() }
}

final class HeldControl: UIButton {
    var onPress: ((Bool) -> Void)?
    private var isInputHeld = false
    private var accessibilityRelease: Task<Void, Never>?

    override func beginTracking(_ touch: UITouch, with event: UIEvent?) -> Bool {
        let accepted = super.beginTracking(touch, with: event)
        if accepted { isInputHeld = true; onPress?(true) }
        return accepted
    }
    override func continueTracking(_ touch: UITouch, with event: UIEvent?) -> Bool {
        if !bounds.contains(touch.location(in: self)) { release() }
        return super.continueTracking(touch, with: event)
    }
    override func endTracking(_ touch: UITouch?, with event: UIEvent?) {
        release(); super.endTracking(touch, with: event)
    }
    override func cancelTracking(with event: UIEvent?) {
        release(); super.cancelTracking(with: event)
    }
    func release() {
        accessibilityRelease?.cancel(); accessibilityRelease = nil
        if isInputHeld { isInputHeld = false; onPress?(false) }
    }
    override func accessibilityActivate() -> Bool {
        guard isEnabled else { return false }
        release(); isInputHeld = true; onPress?(true)
        accessibilityRelease = Task { @MainActor [weak self] in
            do { try await Task.sleep(for: .milliseconds(120)) } catch { return }
            self?.release()
        }
        return true
    }
}
