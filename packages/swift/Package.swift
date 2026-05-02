// swift-tools-version: 6.0
import PackageDescription

// NOTE: Run `cargo build -p kreuzberg-swift` before `swift build`.
// The build step generates Swift + C bridge sources; copy them into Sources/RustBridge
// and Sources/RustBridgeC before building. See BUILDING.md for the full workflow.
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
        // RustBridgeC: pure C/headers target. Swift files in RustBridge import this
        // to access C types (RustStr, etc.) produced by swift-bridge.
        // publicHeadersPath: "." exposes RustBridgeC.h to dependents.
        .target(
            name: "RustBridgeC",
            path: "Sources/RustBridgeC",
            publicHeadersPath: "."
        ),
        // RustBridge: Swift wrapper around the Rust static library.
        // Depends on RustBridgeC so the generated Swift files can use the C types.
        // Note: link the Rust static library by setting LIBRARY_SEARCH_PATHS in your
        // build system rather than unsafeFlags here (unsafeFlags prevents use as a dep).
        .target(
            name: "RustBridge",
            dependencies: ["RustBridgeC"],
            path: "Sources/RustBridge"
        ),
        .target(name: "Kreuzberg", dependencies: ["RustBridge"], path: "Sources/Kreuzberg"),
        .testTarget(name: "KreuzbergTests", dependencies: ["Kreuzberg"], path: "Tests/KreuzbergTests"),
    ]
)
