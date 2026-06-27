```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "chunking": {
        "max_characters": 500,
        "overlap": 50,
        "embedding": {
            "model": {"preset": {"name": "balanced"}},
            "normalize": true
        }
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("research_paper.pdf", nil, config)

if let chunks = result.chunks() {
    for chunk in chunks {
        let metadata = chunk.metadata()
        let content = chunk.content().toString()
        let preview = String(content.prefix(100))
        print("Chunk \(metadata.chunk_index() + 1)/\(metadata.total_chunks())")
        print("Position: \(metadata.byte_start())-\(metadata.byte_end())")
        print("Content: \(preview)...")
        if let embedding = chunk.embedding() {
            print("Embedding: \(embedding.count) dimensions")
        }
    }
}
```
