```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "token_reduction": {
        "mode": "moderate",
        "preserve_markdown": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"verbose_document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

let content = result.content.toString()
print("Reduced content length: \(content.count)")
for warning in result.processing_warnings() {
    print("Warning [\(warning.source().toString())]: \(warning.message().toString())")
}
```
