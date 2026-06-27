```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "enable_quality_processing": true,
    "use_cache": true
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print("Content length: \(result.content.toString().count)")
print("Tables: \(result.tables.count)")
```
