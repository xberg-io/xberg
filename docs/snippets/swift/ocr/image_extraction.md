```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "images": {
        "extract_images": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("document.pdf", nil, config)

print(result.content().toString())
```
