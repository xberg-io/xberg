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
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print("Content length: \(result.content.toString().count)")
```
