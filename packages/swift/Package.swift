// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Kreuzberg",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
    ],
    products: [
        .library(name: "Kreuzberg", targets: ["Kreuzberg"]),
    ],
    targets: [
        .target(name: "Kreuzberg", path: "Sources/Kreuzberg"),
        .testTarget(name: "KreuzbergTests", dependencies: ["Kreuzberg"], path: "Tests/KreuzbergTests"),
    ]
)
