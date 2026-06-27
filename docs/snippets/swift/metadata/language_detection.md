```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "language_detection": {
        "enabled": true,
        "min_confidence": 0.8,
        "detect_multiple": false
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

if let languages = result.detected_languages() {
    let langs = languages.map { $0.toString() }
    print("Detected languages: \(langs)")
} else {
    print("No languages detected")
}
```
