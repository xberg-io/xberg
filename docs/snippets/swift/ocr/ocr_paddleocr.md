```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "ocr": {
        "backend": "paddleocr",
        "language": "en"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("document.pdf", nil, config)

print(result.content().toString())
```
