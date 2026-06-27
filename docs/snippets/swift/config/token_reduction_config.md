```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "token_reduction": {
        "mode": "moderate",
        "preserve_important_words": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

print("Reduced content length: \(result.content().toString().count)")
```
