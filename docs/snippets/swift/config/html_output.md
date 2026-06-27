```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "output_format": "html",
    "html_output": {
        "theme": "github"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print(result.content.toString()) // HTML with kb-* classes
```
