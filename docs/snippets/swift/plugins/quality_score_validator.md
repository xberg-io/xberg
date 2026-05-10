<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: a `Validator` that gates on a `quality_score` metadata field would
// have to implement the Rust `Validator` trait. swift-bridge does not map
// Rust traits onto Swift protocols, so this is not available natively.
//
// Validate post-hoc by reading `ExtractionResult.metadata` after a
// successful `extractFile` / `extractBytes` call. For pipeline-level
// gating, implement the validator in Rust.
```
