```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "enable_quality_processing": true
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"scanned_document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let score = result.quality_score() {
    if score < 0.5 {
        print(String(format: "Warning: Low quality extraction (%.2f)", score))
    } else {
        print(String(format: "Quality score: %.2f", score))
    }
}
```
