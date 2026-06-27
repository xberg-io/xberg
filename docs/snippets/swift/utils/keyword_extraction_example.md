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
let input = try extractInputFromJson(#"{"kind":"uri","uri":"research_paper.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let keywords = result.extracted_keywords() {
    for keyword in keywords {
        let text = keyword.text().toString()
        let score = keyword.score()
        print("\(text) (score: \(score))")
    }
}
```
