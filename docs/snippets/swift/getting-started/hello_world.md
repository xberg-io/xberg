```swift title="Swift"
import Foundation
import Xberg
import RustBridge

print("Hello")

let config = try extractionConfigFromJson("{}")
let result = try extractSync("document.pdf", nil, config)

print("MIME type: \(result.mime_type().toString())")
```
