```swift title="Swift"
import Foundation
import Xberg
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
let result = try extract("multilingual.pdf", nil, config)

print(result.content().toString())
```
