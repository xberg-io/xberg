```swift title="Swift"
import Foundation
import Kreuzberg
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
let result = try extractFileSync("document.pdf", nil, config)

if let languages = result.detected_languages() {
    let langs = languages.map { $0.toString() }
    print("Detected languages: \(langs)")
}
```
