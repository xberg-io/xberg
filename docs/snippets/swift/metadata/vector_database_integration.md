```swift title="Swift"
import Foundation
import Xberg
import RustBridge

struct VectorRecord {
    let id: String
    let content: String
    let embedding: [Float]
    let metadata: [String: String]
}

let configJson = """
{
    "chunking": {
        "max_characters": 512,
        "overlap": 50,
        "embedding": {
            "model": {"preset": {"name": "balanced"}},
            "batch_size": 32,
            "normalize": true
        }
    }
}
"""

let documentId = "doc_001"
let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)

var records: [VectorRecord] = []

if let chunks = result.chunks() {
    for (index, chunk) in chunks.enumerated() {
        guard let embedding = chunk.embedding() else { continue }

        let content = chunk.content().toString()
        let vector = embedding.map { $0 }

        var metadata: [String: String] = [:]
        metadata["document_id"] = documentId
        metadata["chunk_index"] = String(index)
        metadata["content_length"] = String(content.count)

        records.append(VectorRecord(
            id: "\(documentId)_chunk_\(index)",
            content: content,
            embedding: vector,
            metadata: metadata
        ))
    }
}

print("Generated \(records.count) vector records")
```
