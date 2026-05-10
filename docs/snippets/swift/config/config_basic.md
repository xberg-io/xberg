```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "use_cache": true,
    "enable_quality_processing": true
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("document.pdf", nil, config)

print(result.content().toString())
```
