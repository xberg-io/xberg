<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: the Swift binding does not expose plugin registry management.
// `clear_document_extractors`, `clear_post_processors`, `clear_ocr_backends`,
// and `clear_validators` are Rust-only entry points and are not surfaced
// through swift-bridge. Plugin lifecycle is managed in the Rust core; the
// Swift binding consumes whatever extractors/post-processors/validators
// were registered before the binary loaded the kreuzberg library.
//
// To manage plugins, write a Rust helper crate that links kreuzberg and
// expose your own management functions, or drive plugin configuration
// through `ExtractionConfig` fields surfaced by Alef.
```
