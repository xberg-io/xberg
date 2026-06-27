```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "chunking": {
        "max_characters": 1000,
        "overlap": 200
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

if let chunks = result.chunks() {
    print("Chunks: \(chunks.count)")
    for chunk in chunks {
        let metadata = chunk.metadata()
        print("Chunk \(metadata.chunk_index() + 1)/\(metadata.total_chunks())")
    }
}
```
