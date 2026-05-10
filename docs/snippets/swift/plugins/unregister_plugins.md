<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: the Swift binding does not expose plugin removal.
// The Rust API `registry.remove("plugin-name")` operates on a typed
// `Arc<RwLock<Registry<dyn Trait>>>` that swift-bridge cannot represent
// as an opaque Swift class because the trait objects themselves are not
// FFI-friendly.
//
// Plugins must therefore be removed from the Rust core (or a custom
// shim crate) before kreuzberg is loaded by the Swift host process.
```
