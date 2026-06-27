```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "structured_extraction": {
        "schema": {
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "authors": { "type": "array", "items": { "type": "string" } },
                "date": { "type": "string" }
            },
            "required": ["title", "authors", "date"],
            "additionalProperties": false
        },
        "llm": {
            "model": "openai/gpt-4o-mini"
        },
        "strict": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("paper.pdf", nil, config)

if let structured = result.structured_output() {
    print(structured.toString())
}
```
