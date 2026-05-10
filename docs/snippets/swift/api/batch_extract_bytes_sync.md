```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

// `BatchBytesItem` is an opaque swift-bridge class with no public Swift
// constructor — build items from JSON via `batchBytesItemFromJson`.
// `content` must be encoded as a JSON byte array.
func encodeBytesAsJsonArray(_ bytes: [UInt8]) -> String {
    "[" + bytes.map { String($0) }.joined(separator: ",") + "]"
}

let items = RustVec<BatchBytesItem>()

let first = Array("Hello, world!".utf8)
items.push(value: try batchBytesItemFromJson(
    "{\"content\": \(encodeBytesAsJsonArray(first)), \"mime_type\": \"text/plain\"}"
))

let second = Array("# Heading\n\nParagraph text.".utf8)
items.push(value: try batchBytesItemFromJson(
    "{\"content\": \(encodeBytesAsJsonArray(second)), \"mime_type\": \"text/markdown\"}"
))

let config = try extractionConfigFromJson("{}")
let results = try batchExtractBytesSync(items, config)

for (index, result) in results.enumerated() {
    print("Item \(index): \(result.content().toString().count) chars")
}
```
