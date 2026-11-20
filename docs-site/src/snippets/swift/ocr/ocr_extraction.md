```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "ocr": {
        "backend": "tesseract",
        "language": "eng"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"scanned.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().first!

print(result.content().toString())
```
