```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "chunking": {
        "max_characters": 1024,
        "overlap": 100,
        "embedding": {
            "model": {"preset": {"name": "balanced"}},
            "normalize": true,
            "batch_size": 32,
            "show_download_progress": false
        }
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

if let chunks = result.chunks() {
    print("Generated \(chunks.count) chunks")
    for chunk in chunks {
        if let embedding = chunk.embedding() {
            print("Chunk \(chunk.metadata().chunk_index()) -> \(embedding.count)-dim embedding")
        }
    }
}
```
