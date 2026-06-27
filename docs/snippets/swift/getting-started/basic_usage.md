```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let config = try extractionConfigFromJson("{}")
let result = try extractSync("document.pdf", nil, config)

print(result.content().toString())
print("MIME type: \(result.mime_type().toString())")
```
