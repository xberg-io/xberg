```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "token_reduction": {
        "mode": "moderate",
        "preserve_markdown": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("verbose_document.pdf", nil, config)

let content = result.content().toString()
print("Reduced content length: \(content.count)")
for warning in result.processing_warnings() {
    print("Warning [\(warning.source().toString())]: \(warning.message().toString())")
}
```
