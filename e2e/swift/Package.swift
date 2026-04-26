// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "E2eSwift",
    platforms: [
        .macOS(.v13),
    ],
    dependencies: [
        .package(path: "../../packages/swift"),
    ],
    targets: [
        .testTarget(
            name: "KreuzbergTests",
            dependencies: ["Kreuzberg"]
        ),
    ]
)
