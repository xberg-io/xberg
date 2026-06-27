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
let result = try extractSync("document.pdf", nil, config)

if let elements = result.elements() {
    print("Elements: \(elements.count)")
    for element in elements {
        print("Type: \(element.element_type().toString())")
        print("Text: \(element.text().toString().prefix(100))")
    }
}
```
