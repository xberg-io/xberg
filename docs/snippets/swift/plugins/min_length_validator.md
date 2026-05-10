<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: the Rust `Validator` trait cannot be implemented from Swift via
// swift-bridge — `async_trait` methods returning `Result<()>` are not
// expressible as Swift protocol conformances.
//
// To enforce a minimum-length policy from Swift, validate the result
// after extraction returns:
//
//     let result = try extractFile(path: "doc.pdf", config: config)
//     let text = result.content().toString()
//     guard text.count >= 100 else {
//         throw KreuzbergError.validation(
//             message: "Content too short: \(text.count) < 100",
//             source: "min-length-validator"
//         )
//     }
//
// For an in-pipeline validator that runs inside `extract_*`, implement
// the `Validator` trait in Rust.
```
