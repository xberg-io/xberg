```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "keywords": {
        "algorithm": "yake",
        "max_keywords": 10,
        "min_score": 0.1,
        "ngram_range": [1, 3],
        "language": "en"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print("Keywords extracted from document")
print("Content length: \(result.content.toString().count)")
```
