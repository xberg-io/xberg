```swift title="Swift"
import Foundation
import Xberg
import RustBridge

// Custom/cloud OCR backends are registered via the Rust plugin system.
// From Swift, select a registered custom backend by name through the
// JSON configuration:
let configJson = """
{
    "ocr": {
        "backend": "custom",
        "language": "eng"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"scanned.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print(result.content.toString())
```
