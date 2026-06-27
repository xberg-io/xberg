```swift title="Swift"
import Foundation
import Xberg
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
let result = try extractSync("scanned.pdf", nil, config)

if let elements = result.ocr_elements() {
    for element in elements {
        print("Text: \(element.text().toString())")
    }
}
```
