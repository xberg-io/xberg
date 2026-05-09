<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: a "PDF-only" post-processor relies on implementing the Rust
// `PostProcessor` trait with `should_process` returning `true` only for
// `application/pdf`. swift-bridge cannot map Rust traits onto Swift
// protocols, so this pattern is not available from Swift.
//
// Implement the gating logic in Rust against the `PostProcessor` trait
// from `kreuzberg::plugins`, register it in a Rust shim crate, and consume
// the resulting behaviour through the regular `ExtractionConfig` surface
// already exposed to Swift.
```
