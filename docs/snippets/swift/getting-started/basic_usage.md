```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let config = try extractionConfigFromJson("{}")
let result = try extractFileSync("document.pdf", nil, config)

print(result.content().toString())
print("MIME type: \(result.mime_type().toString())")
```
