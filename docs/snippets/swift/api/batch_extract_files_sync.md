```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

// `BatchFileItem` is an opaque swift-bridge class with no public Swift
// constructor — build items from JSON via `batchFileItemFromJson`.
let items = RustVec<BatchFileItem>()
for path in ["doc1.pdf", "doc2.docx", "report.pdf"] {
    let json = "{\"path\": \"\(path)\"}"
    items.push(value: try batchFileItemFromJson(json))
}

let config = try extractionConfigFromJson("{}")
let results = try batchExtractFilesSync(items, config)

for (index, result) in results.enumerated() {
    print("File \(index): \(result.content().toString().count) chars")
}
```
