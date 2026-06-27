```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "token_reduction": {
        "mode": "moderate",
        "preserve_important_words": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print("Reduced content length: \(result.content.toString().count)")
```
