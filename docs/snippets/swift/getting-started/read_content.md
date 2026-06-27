```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "chunking": {
        "max_characters": 800,
        "overlap": 100,
        "chunker_type": "markdown"
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

let tables = result.tables
print("Tables: \(tables.count)")
for (index, _) in tables.enumerated() {
    print("Table \(index)")
}

if let chunks = result.chunks {
    print("Chunks: \(chunks.count)")
    for (index, _) in chunks.enumerated() {
        print("Chunk \(index)")
    }
}
```
