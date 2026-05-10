```swift title="Swift"
import Foundation
import Kreuzberg
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
let result = try extractFileSync("scanned.pdf", nil, config)

print(result.content().toString())
```
