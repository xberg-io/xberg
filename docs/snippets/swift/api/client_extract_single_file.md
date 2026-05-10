```swift title="Swift"
import Foundation
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

@main
struct App {
    static func main() async throws {
        let fileURL = URL(fileURLWithPath: "document.pdf")
        let fileData = try Data(contentsOf: fileURL)
        let fileName = fileURL.lastPathComponent

        let boundary = "Boundary-\(UUID().uuidString)"
        var request = URLRequest(url: URL(string: "http://localhost:8000/extract")!)
        request.httpMethod = "POST"
        request.setValue(
            "multipart/form-data; boundary=\(boundary)",
            forHTTPHeaderField: "Content-Type"
        )

        var body = Data()
        body.append("--\(boundary)\r\n".data(using: .utf8)!)
        body.append(
            "Content-Disposition: form-data; name=\"file\"; filename=\"\(fileName)\"\r\n"
                .data(using: .utf8)!
        )
        body.append("Content-Type: application/pdf\r\n\r\n".data(using: .utf8)!)
        body.append(fileData)
        body.append("\r\n--\(boundary)--\r\n".data(using: .utf8)!)
        request.httpBody = body

        let (data, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw NSError(domain: "kreuzberg", code: 1)
        }

        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
        print(json["content"] as? String ?? "")
    }
}
```
