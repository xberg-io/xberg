```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "include_document_structure": true
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

if let document = result.document() {
    print("Document nodes: \(document.nodes().count)")
}
```
