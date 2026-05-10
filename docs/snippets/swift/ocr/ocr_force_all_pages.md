```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "force_ocr": true,
    "ocr": {
        "backend": "tesseract",
        "language": "eng"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("document.pdf", nil, config)

print(result.content().toString())
```
