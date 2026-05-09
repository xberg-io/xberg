<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: stateful plugins (e.g. counters, caches) rely on implementing
// the Rust `Plugin` + `PostProcessor` traits with interior-mutable state
// (`AtomicUsize`, `Mutex<HashMap>`). swift-bridge cannot express these
// traits as Swift protocols, so stateful plugins must be authored in
// Rust.
//
// If you need stateful processing on the Swift side, accumulate state
// in your Swift code around the synchronous `extractFile` / `extractBytes`
// entry points instead.
```
