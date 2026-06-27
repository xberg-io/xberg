```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "chunking": {
        "max_characters": 1000,
        "overlap": 200,
        "embedding": {
            "model": {"preset": {"name": "balanced"}},
            "batch_size": 16,
            "normalize": true,
            "show_download_progress": true
        }
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

if let chunks = result.chunks() {
    print("Chunks with embeddings: \(chunks.count)")
}
```
