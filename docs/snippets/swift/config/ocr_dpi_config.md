```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "images": {
        "extract_images": true,
        "target_dpi": 300,
        "max_image_dimension": 4096,
        "auto_adjust_dpi": true,
        "min_dpi": 150,
        "max_dpi": 600
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("document.pdf", nil, config)

print("Content length: \(result.content().toString().count)")
```
