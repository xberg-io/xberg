<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: implementing a `PostProcessor` (e.g. a word-count enrichment) is
// a Rust-only capability. swift-bridge cannot bridge `async_trait` methods
// or `&mut ExtractionResult` mutable callbacks back into Swift code.
//
// The closest Swift-side equivalent is to post-process `ExtractionResult`
// after `extractFile` / `extractBytes` returns:
//
//     let result = try extractFile(path: "doc.pdf", config: config)
//     let words = result.content().toString().split(separator: " ").count
//     print("words: \(words)")
//
// For an in-pipeline post-processor, author it in Rust against
// `kreuzberg::plugins::PostProcessor`.
```
