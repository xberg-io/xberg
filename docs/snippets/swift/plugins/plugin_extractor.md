<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: implementing the Rust `DocumentExtractor` trait from Swift is
// not feasible through swift-bridge. The trait requires `async_trait`
// methods returning `Result<ExtractionResult>`, which the swift-bridge
// runtime cannot synthesize from a Swift class.
//
// Authoring a custom extractor must be done in Rust. After implementing
// `Plugin + DocumentExtractor`, register the extractor in a Rust shim
// crate that links both `kreuzberg` and the Swift binding crate before
// the Swift host process loads the dynamic library.
```
