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
      url: "https://github.com/kreuzberg-dev/kreuzberg/releases/download/v__ALEF_SWIFT_VERSION__/Kreuzberg-rs.artifactbundle.zip",
      checksum: "__ALEF_SWIFT_CHECKSUM__"
    ),
    // RustBridge: Swift wrapper module owning the swift-bridge generated
    // sources. Depends on RustBridgeC for C type declarations and on
    // RustBridgeBinary so the linker picks up the static library symbols.
    .target(
      name: "RustBridge",
      dependencies: ["RustBridgeC", "RustBridgeBinary"],
      path: "packages/swift/Sources/RustBridge"
    ),
    .target(
      name: "Kreuzberg",
      dependencies: ["RustBridge", "RustBridgeC"],
      path: "packages/swift/Sources/Kreuzberg"
    ),
  ]
)
