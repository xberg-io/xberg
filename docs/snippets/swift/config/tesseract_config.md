```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "ocr": {
        "backend": "tesseract",
        "language": "eng+deu",
        "tesseract_config": {
            "psm": 6,
            "oem": 3
        }
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("scanned.pdf", nil, config)

print("OCR text: \(result.content().toString())")
```
