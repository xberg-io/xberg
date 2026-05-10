```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "enable_quality_processing": true,
    "use_cache": true
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("document.pdf", nil, config)

print("Content length: \(result.content().toString().count)")
print("Tables: \(result.tables().count)")
```
