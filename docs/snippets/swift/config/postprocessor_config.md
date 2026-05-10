```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "postprocessor": {
        "enabled": true,
        "enabled_processors": ["whitespace_normalizer", "unicode_normalizer"]
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("document.pdf", nil, config)

print("Processed content: \(result.content().toString())")
```
