// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "MultiplayerKit",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [.library(name: "MultiplayerKit", targets: ["MultiplayerKit"])],
    targets: [
        .target(name: "MultiplayerKit"),
        .testTarget(name: "MultiplayerKitTests", dependencies: ["MultiplayerKit"]),
    ]
)
