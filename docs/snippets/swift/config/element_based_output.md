```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

let configJson = """
{
    "result_format": "element_based"
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractFileSync("document.pdf", nil, config)

if let elements = result.elements() {
    print("Elements: \(elements.count)")
    for element in elements {
        print("Type: \(element.element_type().toString())")
        print("Text: \(element.text().toString().prefix(100))")
    }
}
```
