```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "pdf_options": {
        "extract_images": true,
        "passwords": ["password123"],
        "extract_metadata": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("encrypted.pdf", nil, config)

print("Content length: \(result.content().toString().count)")
```
