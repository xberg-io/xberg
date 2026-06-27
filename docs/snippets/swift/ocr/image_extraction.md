```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "images": {
        "extract_images": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

print(result.content().toString())
```
