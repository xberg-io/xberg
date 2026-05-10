```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let data = try Data(contentsOf: URL(fileURLWithPath: "document.pdf"))
let content = RustVec<UInt8>()
for byte in data { content.push(value: byte) }

let config = try extractionConfigFromJson("{}")
let result = try extractBytesSync(content, "application/pdf", config)

print(result.content().toString())
print("Tables: \(result.tables().count)")
```
