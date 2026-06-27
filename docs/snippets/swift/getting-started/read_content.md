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
let result = try extractSync("document.pdf", nil, config)

let tables = result.tables()
print("Tables: \(tables.count)")
for (index, _) in tables.enumerated() {
    print("Table \(index)")
}

if let chunks = result.chunks() {
    print("Chunks: \(chunks.count)")
    for (index, _) in chunks.enumerated() {
        print("Chunk \(index)")
    }
}
```
