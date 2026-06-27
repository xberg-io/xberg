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
let input = try extractInputFromJson(#"{"kind":"uri","uri":"scanned.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print("OCR text: \(result.content.toString())")
```
