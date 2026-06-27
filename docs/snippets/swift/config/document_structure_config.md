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
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let document = result.document() {
    print("Document nodes: \(document.nodes().count)")
}
```
