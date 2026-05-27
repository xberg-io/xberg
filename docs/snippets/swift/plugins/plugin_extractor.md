<!-- snippet:skip reason="swift-bridge 0.1.59 does not expose SwiftDocumentExtractorBox constructor or protocol definition in generated Swift code. Custom extractors must be implemented in Rust and registered via FFI shim." -->
```swift title="Swift"
import Kreuzberg

// Custom DocumentExtractor registration is not available from Swift.
//
// The Rust FFI (packages/swift/rust/src/lib.rs) accepts SwiftDocumentExtractorBox,
// but swift-bridge does not generate the Swift-side protocol definition or
// constructor required to implement and register instances.
//
// Solution: Implement DocumentExtractor in Rust and wrap it in a Rust FFI shim
// that links both `kreuzberg` and the `kreuzberg-swift` package.
```
