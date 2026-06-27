```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "use_cache": true,
    "enable_quality_processing": true,
    "ocr": {
        "backend": "tesseract",
        "language": "eng+deu",
        "tesseract_config": {
            "psm": 6
        }
    },
    "chunking": {
        "max_characters": 1000,
        "overlap": 200
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

print("Content length: \(result.content().toString().count)")
```
