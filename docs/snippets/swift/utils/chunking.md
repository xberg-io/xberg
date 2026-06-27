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
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let chunks = result.chunks {
    print("Chunks: \(chunks.count)")
    for chunk in chunks {
        let metadata = chunk.metadata()
        print("Chunk \(metadata.chunk_index() + 1)/\(metadata.total_chunks())")
    }
}
```
