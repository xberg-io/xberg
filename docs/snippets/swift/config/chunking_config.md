```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "chunking": {
        "max_characters": 1000,
        "overlap": 100,
        "chunker_type": "markdown",
        "prepend_heading_context": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.md", nil, config)

if let chunks = result.chunks() {
    print("Chunks: \(chunks.count)")
    for chunk in chunks {
        let content = chunk.content().toString()
        print("Length: \(content.count)")
    }
}
```
