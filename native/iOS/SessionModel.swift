import Combine
import Foundation
import Network

struct ChatEntry: Equatable, Sendable {
    let player: UInt32
    let text: String
    let at: Double
}

/// One session lifecycle for LAN and hosted routes. The socket is disposable;
/// input and view state cannot leak from an old connection into its replacement.
@MainActor
final class SessionModel: ObservableObject {
    @Published private(set) var status = "Not connected"
    @Published private(set) var detail = "Join the same room on another device or browser."
    @Published private(set) var world: GameWorld?
    @Published private(set) var level: GameLevel?
    @Published private(set) var playerID: UInt32?
    @Published private(set) var active = false
    @Published private(set) var requested = false
    @Published private(set) var appliedSequence: UInt32 = 0
    @Published private(set) var upwardTransitions: UInt64 = 0
    @Published private(set) var revision: UInt32 = 0
    @Published private(set) var controllerID: UInt32?
    @Published private(set) var campaignComplete = false
    @Published private(set) var networkSummary = "No network samples"
    /// Last three quick-chat lines, newest last. Wording is server-owned.
    @Published private(set) var chatLog: [String] = []
    @Published private(set) var latestChat: ChatEntry?
    private var networkStatistics = NetworkStatistics()
    private var lastStatisticsUpdate = 0.0

    private var worker: Task<Void, Never>?
    private var socket: URLSessionWebSocketTask?
    private var session: URLSession?
    private var generation = UUID()
    private var foreground = true
    private var plan: ConnectionPlan?
    private var tokens = ResumeStore()
    private var input = InputState()
    private var sequence: UInt32 = 0
    private var lastFreshSnapshot = ContinuousClock.now
    private var hasSnapshot = false
    private var previousVerticalVelocity = 0
    private var pendingControl: CampaignAction?
    private var controlSequence: UInt32?
    private var awaitingBaseline = true
    private var baselineSequence: UInt32?
    private var preferredRoute: URL?
    private let pathMonitor = NWPathMonitor()
    private var pathRecovery = PathRecoveryState()
    private var lastPathRecovery: ContinuousClock.Instant?
    private var acknowledgmentDeadline = InputAcknowledgmentDeadline()
    private var reachedUsableState = false
    private var playtestCode = ""

    init() {
        pathMonitor.pathUpdateHandler = { [weak self] path in
            let interface: NetworkPathSignal.Interface = path.usesInterfaceType(.wifi) ? .wifi
                : path.usesInterfaceType(.cellular) ? .cellular
                : path.usesInterfaceType(.wiredEthernet) ? .wired : .other
            let signal = NetworkPathSignal(reachable: path.status == .satisfied, interface: interface)
            Task { @MainActor [weak self] in self?.pathChanged(signal) }
        }
        pathMonitor.start(queue: DispatchQueue(label: "coop.network-path"))
    }

    deinit { pathMonitor.cancel() }

    private func pathChanged(_ signal: NetworkPathSignal) {
        guard pathRecovery.observe(signal), requested, foreground else { return }
        // Reachability is only a hint: loopback/local connections can work without
        // Internet. Never permanently block a session based on NWPathMonitor.
        input.clear()
        if let lastPathRecovery, lastPathRecovery.duration(to: .now) < .milliseconds(500) { return }
        lastPathRecovery = .now
        begin()
    }

    private struct Rejected: Error { let code: String }

    func connect(primary: String, fallback: String, room: String, accessCode: String = "") {
        do {
            #if DEBUG
            let localDevelopment = true
            #else
            let localDevelopment = false
            #endif
            let newPlan = try ConnectionPlan(endpoints: [primary, fallback], room: room, allowLocalInsecure: localDevelopment)
            // This is a user action: a previously expired token has already been forgotten.
            let changedSession = plan.map { $0.room != newPlan.room || $0.routes != newPlan.routes } ?? false
            if changedSession, let previous = plan {
                for route in previous.routes { tokens.forget(route, room: previous.room) }
                preferredRoute = nil
            }
            stopSocket(permanent: changedSession)
            plan = newPlan
            let newCode = accessCode.trimmingCharacters(in: .whitespacesAndNewlines)
            if newCode != playtestCode { preferredRoute = nil }
            playtestCode = newCode
            requested = true
            world = nil; level = nil; playerID = nil
            begin()
        } catch {
            if requested {
                detail = "Settings not applied; the existing session is unchanged. Check the endpoint and room."
            } else {
                status = "Check connection settings"
                detail = "Use a 3–16 character room and a wss:// endpoint. Debug builds also allow ws:// on a private LAN."
            }
        }
    }

    func leave() {
        requested = false
        plan = nil
        tokens.clear()
        playtestCode = ""
        preferredRoute = nil
        stopSocket(permanent: true)
        world = nil; level = nil; playerID = nil
        status = "Not connected"
        detail = "You left the room."
    }

    func setForeground(_ value: Bool) {
        guard foreground != value else { return }
        foreground = value
        if value {
            if requested { begin() }
        } else {
            stopSocket(permanent: false)
            if requested { status = "Paused"; detail = "Input released. The session will resume when you return." }
        }
    }

    func set(_ action: InputState.Action, source: String, pressed: Bool) {
        guard active && foreground && !campaignComplete else { return }
        input.set(action, source: source, pressed: pressed)
    }

    func releaseControls() { input.clear() }

    func sendChat(preset: UInt8) {
        guard let socket, active || hasSnapshot else { return }
        Task { [weak socket] in
            guard let data = try? ClientCommand.chat(preset: preset) else { return }
            try? await socket?.send(.string(String(decoding: data, as: UTF8.self)))
        }
    }

    func restartLevel() { request(.restart) }
    func nextLevel() { request(.next) }

    private func request(_ action: CampaignAction) {
        guard active, foreground, controllerID == playerID, pendingControl == nil else { return }
        if action == .next && (world?.completed != true || campaignComplete) { return }
        input.clear()
        pendingControl = action
        campaignComplete = false
        lastFreshSnapshot = .now
        active = false
    }

    private func stopSocket(permanent: Bool) {
        generation = UUID()
        worker?.cancel(); worker = nil
        socket?.cancel(with: permanent ? .normalClosure : .goingAway, reason: nil)
        socket = nil
        session?.invalidateAndCancel(); session = nil
        input.clear()
        active = false
        hasSnapshot = false
        pendingControl = nil; controlSequence = nil
        awaitingBaseline = true; baselineSequence = nil
        acknowledgmentDeadline.clear()
        latestChat = nil
    }

    private func begin() {
        guard foreground, requested, let plan else { return }
        stopSocket(permanent: false)
        let current = generation
        worker = Task { [weak self] in await self?.run(plan, generation: current) }
    }

    private func run(_ plan: ConnectionPlan, generation current: UUID) async {
        let preferred = plan.routes.firstIndex(where: { $0.url == preferredRoute }) ?? 0
        var recovery = RecoveryState(routeIndex: preferred)
        var retrying = false
        while !Task.isCancelled && current == generation && foreground && requested {
            let route = plan.routes[recovery.routeIndex]
            status = retrying ? "Reconnecting…" : "Connecting…"
            detail = "Route \(recovery.routeIndex + 1)/\(plan.routes.count) · \(route.url.host ?? "host")"
            do {
                try await runConnection(route, room: plan.room, token: tokens.token(for: route, room: plan.room), generation: current)
            } catch {
                guard !Task.isCancelled, current == generation else { return }
                recovery.failed(afterHealthyConnection: reachedUsableState, routeCount: plan.routes.count)
                active = false
                input.clear()
                if let rejection = error as? Rejected {
                    if rejection.code == "access_denied" {
                        status = "Access code required or incorrect"
                        detail = "Check the hosted playtest access code, then tap Join."
                        requested = false
                        return
                    }
                    if rejection.code == "invalid_resume_token" {
                        tokens.forget(route, room: plan.room)
                        status = "Session expired"
                        detail = "Tap Join to enter as a new player."
                        requested = false
                        return
                    }
                    if ["protocol_mismatch", "ruleset_mismatch", "invalid_room", "invalid_message"].contains(rejection.code) {
                        status = "Server rejected this client"
                        detail = rejection.code.replacingOccurrences(of: "_", with: " ")
                        requested = false
                        return
                    }
                }
                if error is ProtocolError || error is DecodingError {
                    status = "Incompatible server data"
                    detail = "Stopped safely. Verify the server and client versions before joining again."
                    requested = false
                    return
                }
                status = "Connection interrupted"
                detail = "Controls released. Retrying the same session; Leave stops retries."
            }
            guard current == generation, !Task.isCancelled else { return }
            let delay = plan.retryDelay(attempt: recovery.failures, jitter: Double.random(in: 0...1))
            do { try await Task.sleep(for: .seconds(delay)) } catch { return }
            retrying = true
        }
    }

    private func runConnection(_ route: ConnectionRoute, room: String, token: String?, generation current: UUID) async throws {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 5
        configuration.timeoutIntervalForResource = 10
        configuration.waitsForConnectivity = false
        let session = URLSession(configuration: configuration)
        var request = URLRequest(url: route.url)
        if let primary = plan?.routes.first, let header = PlaytestAccess.header(code: playtestCode, route: route, primary: primary) {
            request.setValue(header, forHTTPHeaderField: "Authorization")
        }
        let task = session.webSocketTask(with: request)
        task.maximumMessageSize = 65_536
        self.session = session; self.socket = task
        sequence = 0
        appliedSequence = 0
        upwardTransitions = 0
        previousVerticalVelocity = 0
        revision = 0; controllerID = nil; campaignComplete = false
        pendingControl = nil; controlSequence = nil
        awaitingBaseline = true; baselineSequence = nil
        acknowledgmentDeadline.clear()
        reachedUsableState = false
        input = InputState()
        hasSnapshot = false
        lastFreshSnapshot = .now
        var stream = SnapshotStream()
        var wireDecoder = SnapshotWireDecoder()
        networkStatistics.beginConnection(at: ProcessInfo.processInfo.systemUptime)
        task.resume()
        // This watchdog is independent of receive/send: neither can hang the client forever.
        let watchdog = Task { [weak self] in
            while !Task.isCancelled {
                do { try await Task.sleep(for: .milliseconds(100)) } catch { return }
                guard let self, current == self.generation else { return }
                let now = ProcessInfo.processInfo.systemUptime
                if now - self.lastStatisticsUpdate >= 1 {
                    self.networkSummary = self.networkStatistics.summary(at: now)
                    self.lastStatisticsUpdate = now
                }
                let age = self.lastFreshSnapshot.duration(to: .now)
                if self.acknowledgmentDeadline.expired(at: .now) {
                    self.active = false
                    self.input.clear()
                    self.status = "Input synchronization timed out"
                    task.cancel(with: .goingAway, reason: nil)
                    return
                }
                if self.hasSnapshot && !self.campaignComplete && age > .milliseconds(750) {
                    self.active = false
                    self.input.clear()
                    self.status = "Waiting for authority…"
                }
                if age > .seconds(self.campaignComplete ? 15 : 5) {
                    task.cancel(with: .goingAway, reason: nil)
                    return
                }
            }
        }
        let sender = Task { [weak self] in
            var cadence = InputSendPolicy()
            var lastFinalHeartbeat = -Double.infinity
            while !Task.isCancelled {
                do { try await Task.sleep(for: .milliseconds(17)) } catch { return }
                guard let self, current == self.generation else { return }
                guard self.hasSnapshot else { continue }
                let now = ProcessInfo.processInfo.systemUptime
                if self.campaignComplete && self.pendingControl == nil {
                    guard now - lastFinalHeartbeat >= 10 else { continue }
                    lastFinalHeartbeat = now
                }
                if self.sequence == .max || self.input.jumpID == .max {
                    task.cancel(with: .goingAway, reason: nil)
                    return
                }
                    let sampled = self.active && self.foreground && !self.campaignComplete ? self.input.sample() : GameInput()
                guard cadence.shouldSend(sampled, jumpID: self.input.jumpID, revision: self.revision,
                                         at: ProcessInfo.processInfo.systemUptime, force: self.pendingControl != nil) else { continue }
                self.sequence += 1
                do {
                    let data: Data
                    if let action = self.pendingControl, !self.awaitingBaseline {
                        self.pendingControl = nil
                        self.controlSequence = self.sequence
                        data = try ClientCommand.control(action: action, seq: self.sequence, revision: self.revision)
                    } else {
                        if self.awaitingBaseline && self.baselineSequence == nil {
                            self.baselineSequence = self.sequence
                        }
                        data = try ClientCommand.input(seq: self.sequence, input: sampled, jumpID: self.input.jumpID, revision: self.revision)
                        self.input.markTransmitted()
                    }
                    self.acknowledgmentDeadline.offer(sequence: self.sequence, at: .now)
                    try await task.send(.string(String(decoding: data, as: UTF8.self)))
                    self.networkStatistics.sent(bytes: data.count, sequence: self.sequence, at: ProcessInfo.processInfo.systemUptime)
                } catch {
                    task.cancel(with: .goingAway, reason: nil)
                    return
                }
            }
        }
        defer {
            sender.cancel(); watchdog.cancel()
            task.cancel(with: .goingAway, reason: nil)
            session.invalidateAndCancel()
            if current == generation { self.socket = nil; self.session = nil; active = false }
        }
        do {
        let hello = try ClientCommand.hello(room: room, resumeToken: token, patches: true)
        try await task.send(.string(String(decoding: hello, as: UTF8.self)))
        networkStatistics.sent(bytes: hello.count, at: ProcessInfo.processInfo.systemUptime)
        while !Task.isCancelled && current == generation {
            let message = try await task.receive()
            guard current == generation, !Task.isCancelled else { return }
            let data: Data
            switch message {
            case .string(let text): data = Data(text.utf8)
            case .data: throw ProtocolError.unknownMessage // This protocol explicitly uses text frames.
            @unknown default: throw ProtocolError.unknownMessage
            }
            networkStatistics.received(bytes: data.count)
            switch try wireDecoder.decode(data) {
            case .welcome(let welcome):
                try stream.start(welcome)
                tokens.save(welcome.resume_token, for: route, room: room)
                level = welcome.level; playerID = welcome.player
                revision = welcome.revision; controllerID = welcome.controller
                detail = welcome.resumed ? "Session restored · P\(welcome.player)" : "Joined · P\(welcome.player)"
            case .snapshot(let snapshot):
                guard try stream.accept(snapshot, lastSentSequence: sequence) else { continue }
                if snapshot.revision != revision {
                    input.clear()
                    pendingControl = nil; controlSequence = nil
                    awaitingBaseline = true; baselineSequence = nil
                    acknowledgmentDeadline.clear()
                    previousVerticalVelocity = 0
                    upwardTransitions = 0
                }
                revision = snapshot.revision
                level = snapshot.level
                controllerID = snapshot.controller
                campaignComplete = snapshot.campaign_complete
                if let local = snapshot.world.players.first(where: { $0.id == playerID }) {
                    // Persistent diagnostic evidence of authoritative motion. A short
                    // jump may begin and end while XCTest waits for the UI to idle.
                    if local.vy < 0 && previousVerticalVelocity >= 0 && upwardTransitions < .max {
                        upwardTransitions += 1
                    }
                    previousVerticalVelocity = local.vy
                }
                world = snapshot.world
                appliedSequence = snapshot.acks.first(where: { $0.player == playerID })?.seq ?? 0
                networkStatistics.snapshot(ack: appliedSequence, at: ProcessInfo.processInfo.systemUptime)
                acknowledgmentDeadline.acknowledge(sequence: appliedSequence)
                if let baselineSequence, appliedSequence >= baselineSequence { awaitingBaseline = false }
                if let controlSequence, appliedSequence >= controlSequence { self.controlSequence = nil }
                lastFreshSnapshot = .now
                hasSnapshot = true
                active = !awaitingBaseline && pendingControl == nil && controlSequence == nil
                if active {
                    reachedUsableState = true
                    preferredRoute = route.url
                }
                status = campaignComplete ? "Campaign complete!" : snapshot.world.completed ? "Team complete!" :
                    active ? "Connected · P\(playerID ?? 0)" : "Synchronizing controls…"
            case .chat(let message):
                latestChat = ChatEntry(player: message.player, text: message.text, at: ProcessInfo.processInfo.systemUptime)
                chatLog.append("P\(message.player): \(message.text)")
                if chatLog.count > 3 { chatLog.removeFirst(chatLog.count - 3) }
            case .rejection(let code, _): throw Rejected(code: code)
            }
        }
        } catch {
            if (task.response as? HTTPURLResponse)?.statusCode == 401 { throw Rejected(code: "access_denied") }
            throw error
        }
    }
}
