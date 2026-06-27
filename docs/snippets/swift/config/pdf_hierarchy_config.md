```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "pdf_options": {
        "hierarchy": {
            "enabled": true,
            "detection_threshold": 0.75,
            "ocr_coverage_threshold": 0.8,
            "min_level": 1,
            "max_level": 5
        }
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print("Content length: \(result.content.toString().count)")
```
