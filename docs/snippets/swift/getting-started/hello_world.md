```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

print("Hello")

let config = try extractionConfigFromJson("{}")
let result = try extractFileSync("document.pdf", nil, config)

print("MIME type: \(result.mime_type().toString())")
```
