<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: the Swift binding does not expose `register_document_extractor` /
// `get_document_extractor_registry`. swift-bridge cannot bridge Rust traits
// like `DocumentExtractor` to native Swift protocols, so there is no way
// to construct a Swift-side extractor and hand it to the Rust registry.
//
// Built-in extractors (PDF, DOCX, HTML, etc.) are registered automatically
// by the kreuzberg core when the library initializes. Custom extractors
// must be written in Rust and linked into a custom build of the binding.
```
