```swift title="Swift"
import Foundation
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

@main
struct App {
    static func main() async throws {
        let payload: [String: Any] = [
            "text": "Your long text content here...",
            "chunker_type": "text",
            "config": [
                "max_characters": 1000,
                "overlap": 50,
                "trim": true,
            ],
        ]

        var request = URLRequest(url: URL(string: "http://localhost:8000/chunk")!)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: payload)

        let (data, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw NSError(domain: "kreuzberg", code: 1)
        }

        let result = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
        let chunkCount = result["chunk_count"] as? Int ?? 0
        print("Created \(chunkCount) chunks")

        if let chunks = result["chunks"] as? [[String: Any]] {
            for chunk in chunks {
                let content = chunk["content"] as? String ?? ""
                let index = chunk["chunk_index"] as? Int ?? 0
                let preview = String(content.prefix(50))
                print("Chunk \(index): \(preview)...")
            }
        }
    }
}
```
