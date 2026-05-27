<!-- snippet:skip reason="swift-bridge does not generate Swift-side protocol constructors for plugin registration. The Rust-side FFI defines SwiftDocumentExtractorBox as an opaque extern \"Swift\" type, but swift-bridge does not surface the protocol definition or constructor in the generated Swift package. Custom implementations must be written in Rust." -->
```swift title="Swift"
import Kreuzberg

// Custom DocumentExtractor registration is not available from Swift.
//
// The Rust FFI defines SwiftDocumentExtractorBox as an opaque extern "Swift" type
// (packages/swift/rust/src/lib.rs, lines 2710-2722), but the swift-bridge code
// generator does not emit a Swift-side protocol definition or factory to construct
// and register instances.
//
// Workaround: Implement DocumentExtractor in Rust and register via a Rust FFI shim,
// or use the built-in extractors (PDF, DOCX, HTML, etc.) which are pre-registered.
```
