```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "keywords": {
        "algorithm": "yake",
        "max_keywords": 10,
        "min_score": 0.3
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("research_paper.pdf", nil, config)

if let keywords = result.extracted_keywords() {
    for keyword in keywords {
        let text = keyword.text().toString()
        let score = keyword.score()
        print("\(text) (score: \(score))")
    }
}
```
