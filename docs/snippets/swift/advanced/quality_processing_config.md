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
let result = try extractSync("document.pdf", nil, config)

if let score = result.quality_score() {
    print(String(format: "Quality score: %.2f", score))
}
```
