```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "pdf_options": {
        "extract_images": true,
        "passwords": ["password123"],
        "extract_metadata": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"encrypted.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print("Content length: \(result.content.toString().count)")
```
