```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let config = try extractionConfigFromJson("{}")
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print(result.content.toString())
print("MIME type: \(result.mimeType.toString())")
```
