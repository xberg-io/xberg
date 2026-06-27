```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "output_format": "html",
    "html_output": {
        "theme": "github"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

print(result.content().toString()) // HTML with kb-* classes
```
