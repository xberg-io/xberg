```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "language_detection": {
        "enabled": true,
        "min_confidence": 0.8,
        "detect_multiple": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"multilingual_document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let languages = result.detectedLanguages {
    let langs = languages.map { $0.toString() }
    print("Detected languages: \(langs)")
}
```
