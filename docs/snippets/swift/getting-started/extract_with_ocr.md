```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let input = try extractInputFromJson(#"{"kind":"uri","uri":"scanned.pdf"}"#)
let configJson = """
{
    "force_ocr": true,
    "ocr": {
        "enabled": true,
        "backend": "tesseract",
        "language": ["eng"]
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let output = try await extract(input: input, config: config)

if let document = output.results[0] {
    print(document.content.toString())
    print("MIME type: \(document.mimeType.toString())")
}
```
