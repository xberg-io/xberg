```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "images": {
        "extract_images": true,
        "target_dpi": 300,
        "max_image_dimension": 2000
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

print(result.content().toString())
```
