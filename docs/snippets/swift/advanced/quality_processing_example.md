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
let result = try extractSync("scanned_document.pdf", nil, config)

if let score = result.quality_score() {
    if score < 0.5 {
        print(String(format: "Warning: Low quality extraction (%.2f)", score))
    } else {
        print(String(format: "Quality score: %.2f", score))
    }
}
```
