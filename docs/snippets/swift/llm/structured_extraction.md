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
let input = try extractInputFromJson(#"{"kind":"uri","uri":"paper.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let structured = result.structured_output() {
    print(structured.toString())
}
```
