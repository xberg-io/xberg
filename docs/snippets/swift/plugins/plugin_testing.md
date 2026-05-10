<!-- snippet:skip -->
```swift title="Swift"
import Kreuzberg
import RustBridge

// Note: unit-testing a custom plugin from Swift requires the plugin to
// be implementable in Swift. swift-bridge does not bridge the Rust
// `DocumentExtractor` / `PostProcessor` / `Validator` traits, so plugin
// implementations and their unit tests must live in Rust.
//
// What you *can* test from Swift is end-to-end behaviour of the public
// API after any Rust-registered plugins have been loaded. For example,
// using XCTest:
//
//     import XCTest
//     @testable import Kreuzberg
//
//     final class ExtractionTests: XCTestCase {
//         func testJsonExtraction() throws {
//             let json = "{\"message\": \"Hello, world!\"}"
//             let config = try extractionConfigFromJson("{}")
//             let result = try extractBytes(
//                 content: json,
//                 mimeType: "application/json",
//                 config: config
//             )
//             XCTAssertTrue(result.content().toString().contains("Hello, world!"))
//             XCTAssertEqual(result.mime_type().toString(), "application/json")
//         }
//     }
```
