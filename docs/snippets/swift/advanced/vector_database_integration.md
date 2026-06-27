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

func extractAndVectorize(documentPath: String, documentId: String) async throws -> [VectorRecord] {
    let configJson = """
    {
        "chunking": {
            "max_characters": 512,
            "overlap": 50,
            "embedding": {
                "model": {"preset": {"name": "balanced"}},
                "normalize": true,
                "batch_size": 32
            }
        }
    }
    """

    let config = try extractionConfigFromJson(configJson)
    let input = try extractInputFromJson(#"{"kind":"uri","uri":"\#(documentPath)"}"#)
    let resultOutput = try await extract(input: input, config: config)
    let result = resultOutput.results().get(index: 0)!

    var records: [VectorRecord] = []
    if let chunks = result.chunks {
        for (index, chunk) in chunks.enumerated() {
            guard let embedding = chunk.embedding() else { continue }
            let content = chunk.content().toString()
            let metadata: [String: String] = [
                "document_id": documentId,
                "chunk_index": String(index),
                "content_length": String(content.count),
            ]
            records.append(VectorRecord(
                id: "\(documentId)_chunk_\(index)",
                content: content,
                embedding: embedding.map { $0 },
                metadata: metadata
            ))
        }
    }
    return records
}
```
