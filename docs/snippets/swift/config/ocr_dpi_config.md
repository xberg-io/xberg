```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "images": {
        "extract_images": true,
        "target_dpi": 300,
        "max_image_dimension": 4096,
        "auto_adjust_dpi": true,
        "min_dpi": 150,
        "max_dpi": 600
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print("Content length: \(result.content.toString().count)")
```
