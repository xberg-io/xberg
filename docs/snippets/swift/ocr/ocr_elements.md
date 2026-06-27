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
let input = try extractInputFromJson(#"{"kind":"uri","uri":"scanned.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let elements = result.ocr_elements() {
    for element in elements {
        print("Text: \(element.text().toString())")
    }
}
```
