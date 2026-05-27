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
    // RustBridge: pre-built binary target containing the compiled Rust library
    // for macOS (arm64, x86_64), iOS (device, simulator), and Linux (arm64, x86_64).
    // The binary includes C headers for swift-bridge interop.
    .binaryTarget(
      name: "RustBridge",
      url: "https://github.com/kreuzberg-dev/kreuzberg/releases/download/v__ALEF_SWIFT_VERSION__/Kreuzberg-rs.artifactbundle.zip",
      checksum: "__ALEF_SWIFT_CHECKSUM__"
    ),
    .target(
      name: "Kreuzberg",
      dependencies: ["RustBridge"],
      path: "packages/swift/Sources/Kreuzberg"
    ),
  ]
)
