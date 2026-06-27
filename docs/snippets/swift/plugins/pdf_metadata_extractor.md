<!-- snippet:skip reason="swift-bridge 0.1.59 does not expose SwiftDocumentExtractorBox constructor or protocol definition in generated Swift code. Custom extractors must be implemented in Rust and registered via FFI shim." -->
```swift title="Swift"
import Xberg

// Custom DocumentExtractor registration is not available from Swift.
//
// The FFI defines SwiftDocumentExtractorBox opaque type (packages/swift/rust/src/lib.rs),
// but swift-bridge's Swift code generator does not emit the protocol definition or
// factory required to construct and register instances from Swift.
//
// Workaround: Augment PDF extraction results by implementing a PostProcessor in Rust,
// or post-process ExtractedDocument.metadata in Swift after extraction.
```
