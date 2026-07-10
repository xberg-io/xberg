// swift-tools-version: 6.0
// Root-level Package.swift — alef-generated for published distributions.
//
// This manifest uses `.binaryTarget` for pre-built XCFramework/artifact bundles.
// External consumers depend on this via `.package(url: "...", from: "...")`.
//
// For in-tree development, see `packages/swift/Package.swift` and
// `packages/swift/README.md` for the source-based workflow.
import PackageDescription

let package = Package(
  name: "Xberg",
  platforms: [
    .macOS(.v13),
    .iOS(.v16),
  ],
  products: [
    .library(name: "Xberg", targets: ["Xberg"])
  ],
  targets: [
    // RustBridgeC: C headers target. Swift files in RustBridge import this to
    // access C types (RustStr, etc.) produced by swift-bridge.
    // publicHeadersPath: "." exposes the headers.
    .target(
      name: "RustBridgeC",
      path: "packages/swift/Sources/RustBridgeC",
      publicHeadersPath: "."
    ),
    // RustBridgeBinary: pre-built static library for macOS (arm64, x86_64),
    // iOS (device, simulator), and Linux (arm64, x86_64). The artifactbundle
    // ships `.a` files only — SwiftPM binary targets cannot supply Swift
    // modules, so the swift-bridge generated Swift sources live in the
    // sibling RustBridge target below and link against this binary.
    .binaryTarget(
      name: "RustBridgeBinary",
      url: "https://github.com/xberg-io/xberg/releases/download/v1.0.0-rc.18/Xberg-rs.artifactbundle.zip",
      checksum: "__ALEF_SWIFT_CHECKSUM__"
    ),
    // RustBridge: Swift wrapper module owning the swift-bridge generated
    // sources. Depends on RustBridgeC for C type declarations and on
    // RustBridgeBinary so the linker picks up the static library symbols.
    .target(
      name: "RustBridge",
      dependencies: ["RustBridgeC", "RustBridgeBinary"],
      path: "packages/swift/Sources/RustBridge",
      // The pre-built static library inside RustBridgeBinary references Apple
      // system frameworks (e.g. reqwest's proxy detection pulls in the Rust
      // `system_configuration` crate → `SC*` symbols). The artifactbundle ships
      // only the `.a`, so these frameworks must be linked by the consumer.
      linkerSettings: [
        .linkedFramework("Security", .when(platforms: [.macOS, .iOS])),
        .linkedFramework("CoreFoundation", .when(platforms: [.macOS, .iOS])),
        .linkedFramework("SystemConfiguration", .when(platforms: [.macOS])),
      ]
    ),
    .target(
      name: "Xberg",
      dependencies: ["RustBridge", "RustBridgeC"],
      path: "packages/swift/Sources/Xberg"
    ),
  ]
)
