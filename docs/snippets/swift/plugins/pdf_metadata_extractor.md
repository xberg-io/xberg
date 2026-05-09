<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: a custom PDF metadata extractor would have to implement the Rust
// `DocumentExtractor` trait. swift-bridge does not bridge Rust traits to
// Swift protocols, so this pattern cannot be expressed in Swift.
//
// PDF metadata is already populated on `ExtractionResult.metadata` by the
// built-in PDF extractor — see the `Metadata` and `PdfMetadata` typealiases
// re-exported from `RustBridge`. To project additional fields, write a
// post-processor in Rust and surface its configuration through
// `ExtractionConfig`.
```
