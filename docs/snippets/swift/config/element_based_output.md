```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "result_format": "element_based"
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let elements = result.elements() {
    print("Elements: \(elements.count)")
    for element in elements {
        print("Type: \(element.element_type().toString())")
        print("Text: \(element.text().toString().prefix(100))")
    }
}
```
