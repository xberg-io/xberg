<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: the kreuzberg `Validator` plugin trait is Rust-only. swift-bridge
// does not bridge Rust traits to Swift protocols, and there is no
// `register_validator` symbol exposed by `Kreuzberg.swift`.
//
// Author validators in Rust (`impl Plugin + Validator for MyValidator`)
// and register them through a Rust shim crate before the Swift host
// process loads the dynamic library.
```
