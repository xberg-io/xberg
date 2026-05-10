<!-- snippet:syntax-only -->
```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

// Standalone embedding requires constructing an EmbeddingConfig directly.
// The Swift bindings expose `embedTexts` / `embedTextsAsync`, but
// EmbeddingConfig is an opaque proxy class — no JSON-config decoding is
// available. Build it via the generated initializer or use chunking-time
// embedding via `extractionConfigFromJson` (see embedding_with_chunking).
let texts = RustVec<RustString>()
texts.push(value: "Hello, world!".intoRustString())
texts.push(value: "Kreuzberg is fast".intoRustString())

// `config` here is a fully-constructed EmbeddingConfig built via the
// generated initializer in RustBridge.
let embeddings = try embedTexts(texts, config)
print(embeddings.toString())
```
