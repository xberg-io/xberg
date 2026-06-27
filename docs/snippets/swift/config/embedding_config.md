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
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let chunks = result.chunks {
    print("Chunks with embeddings: \(chunks.count)")
}
```
