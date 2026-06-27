```swift title="Swift"
import Foundation
import Xberg
import RustBridge

func extractText(bytes: [UInt8], mimeType: String) throws -> String {
    let content = RustVec<UInt8>()
    for byte in bytes { content.push(value: byte) }
    let config = try extractionConfigFromJson("{}")
    let result = try extractSync(content, mimeType, config)
    return result.content().toString()
}

let data = (try? Data(contentsOf: URL(fileURLWithPath: "document.pdf"))) ?? Data()
let bytes = Array(data)

do {
    let text = try extractText(bytes: bytes, mimeType: "application/pdf")
    print("Extracted \(text.count) chars")
} catch let error as RustString {
    let message = error.toString()
    if message.contains("UnsupportedFormat") {
        print("Format not supported: \(message)")
    } else if message.contains("Ocr") {
        print("OCR failed: \(message)")
    } else {
        print("Error: \(message)")
    }
} catch {
    print("Unexpected error: \(error)")
}
```
