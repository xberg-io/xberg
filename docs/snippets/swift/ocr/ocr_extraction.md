```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "ocr": {
        "backend": "tesseract",
        "language": "eng"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("scanned.pdf", nil, config)

print(result.content().toString())
```
