```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "keywords": {
        "algorithm": "yake",
        "max_keywords": 10,
        "min_score": 0.3,
        "ngram_range": [1, 3],
        "language": "en"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

if let keywords = result.extracted_keywords() {
    print("Extracted \(keywords.count) keywords")
}
```
