<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: `register_embedding_backend` is a Rust-only entry point.
// The `EmbeddingBackend` trait requires async trait methods returning
// `Result<Vec<Vec<f32>>>`, which swift-bridge cannot synthesize from
// a Swift class.
//
// The `EmbeddingModelType.plugin(name:)` enum case *is* exposed in Swift
// (see `Kreuzberg.swift`), so once a backend named `"my-embedder"` has
// been registered from Rust, Swift code can reference it via
// `EmbeddingConfig`:
//
//     let embedConfig = EmbeddingConfig(...)  // built via Alef-generated init
//     // model: .plugin(name: "my-embedder")
//
// To register the backend itself, do so from a Rust shim crate that
// links `kreuzberg` and calls `register_embedding_backend(...)` during
// process startup.
```
