```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "chunking": {
        "max_characters": 500,
        "overlap": 50
    },
    "pages": {
        "extract_pages": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

if let chunks = result.chunks() {
    for chunk in chunks {
        let metadata = chunk.metadata()
        let content = chunk.content().toString()
        let preview = String(content.prefix(50))
        if let first = metadata.first_page(), let last = metadata.last_page() {
            let pageRange = first == last ? "Page \(first)" : "Pages \(first)-\(last)"
            print("Chunk: \(preview)... (\(pageRange))")
        } else {
            print("Chunk: \(preview)... (no page info)")
        }
    }
}
```
