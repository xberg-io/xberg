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
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let score = result.quality_score() {
    print(String(format: "Quality score: %.2f", score))
}
```
