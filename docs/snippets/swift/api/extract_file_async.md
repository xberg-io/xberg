```swift title="Swift"
import Foundation
import Kreuzberg
import RustBridge

@main
struct App {
    static func main() async throws {
        let config = try extractionConfigFromJson("{}")
        // The Swift binding exposes async-compatible entrypoints; even though
        // the bridge calls are synchronous internally, callers may `await` them
        // to integrate with Swift Concurrency.
        let result = try await extractFile("document.pdf", nil, config)

        print(result.content().toString())
        print("MIME type: \(result.mime_type().toString())")
        print("Tables: \(result.tables().count)")
    }
}
```
