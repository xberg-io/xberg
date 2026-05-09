<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: the Swift binding does not expose plugin registry inspection.
// `get_document_extractor_registry`, `get_post_processor_registry`,
// `get_ocr_backend_registry`, and `get_validator_registry` are Rust-only
// APIs that return registry handles holding `Arc<dyn Trait>` values.
// swift-bridge cannot bridge `Arc<dyn Trait>` registries, so these
// listing helpers are intentionally absent from `Kreuzberg.swift`.
//
// If you need runtime introspection of registered plugins, expose the
// listing through your own Rust shim crate and bridge a serializable
// `[String]` of plugin names back to Swift.
```
