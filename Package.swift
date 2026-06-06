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
  name: "Kreuzberg",
  platforms: [
    .macOS(.v13),
    .iOS(.v16),
  ],
  products: [
    .library(name: "Kreuzberg", targets: ["Kreuzberg"])
  ],
  targets: [
    // RustBridgeC: C headers target extracted from the artifact bundle.
    // Swift files in RustBridge import this to access C types (RustStr, etc.)
    // produced by swift-bridge. publicHeadersPath: "." exposes the headers.
    .target(
      name: "RustBridgeC",
      path: "packages/swift/Sources/RustBridgeC",
      publicHeadersPath: "."
    ),
    // RustBridge: pre-built binary target containing the compiled Rust library
    // for macOS (arm64, x86_64), iOS (device, simulator), and Linux (arm64, x86_64).
    // Depends on RustBridgeC so generated Swift files can use the C types.
    .binaryTarget(
      name: "RustBridge",
      url: "https://github.com/kreuzberg-dev/kreuzberg/releases/download/v5.0.0-rc.3/Kreuzberg-rs.artifactbundle.zip",
      checksum: "__ALEF_SWIFT_CHECKSUM__"
    ),
    .target(
      name: "Kreuzberg",
      dependencies: ["RustBridge", "RustBridgeC"],
      path: "packages/swift/Sources/Kreuzberg"
    ),
  ]
)
