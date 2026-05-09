<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: in-plugin logging via the Rust `log` crate is only meaningful
// from inside a Rust plugin implementation. Because Swift cannot
// implement the `Plugin` / `DocumentExtractor` / `PostProcessor` /
// `Validator` traits through swift-bridge, there is no Swift-side
// "plugin logging" pattern.
//
// To trace extraction from Swift, log around the public entry points:
//
//     print("extracting \(path)")
//     let result = try extractFile(path: path, config: config)
//     print("extracted \(result.content().toString().count) chars")
//
// Internal Rust-side `log` records emitted by built-in plugins are
// routed through whatever `log` backend the embedding host installs.
```
