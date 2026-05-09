// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Dsgo",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "dsgo", targets: ["Dsgo"]),
    ],
    targets: [
        .executableTarget(
            name: "Dsgo",
            swiftSettings: [.enableUpcomingFeature("StrictConcurrency")]
        ),
    ]
)
