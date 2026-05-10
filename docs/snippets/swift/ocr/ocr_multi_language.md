```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "ocr": {
        "backend": "tesseract",
        "language": "eng+deu+fra"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("multilingual.pdf", nil, config)

print(result.content().toString())
```
