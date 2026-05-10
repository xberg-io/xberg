```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "ocr": {
        "backend": "paddleocr",
        "language": "en",
        "element_config": {
            "include_elements": true
        }
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("scanned.pdf", nil, config)

if let elements = result.ocr_elements() {
    for element in elements {
        print("Text: \(element.text().toString())")
    }
}
```
