```swift title="Swift"
import Foundation
import Xberg
import RustBridge

func extractText(bytes: [UInt8], mimeType: String) async throws -> String {
    let bytesJson = bytes.map(String.init).joined(separator: ",")
    let input = try extractInputFromJson(
        #"{"kind":"bytes","bytes":[\#(bytesJson)],"mime_type":"\#(mimeType)","filename":"document.pdf"}"#
    )
    let config = try extractionConfigFromJson("{}")
    let output = try await extract(input: input, config: config)
    let document = output.results.get(index: 0)!
    return document.content.toString()
}

let data = (try? Data(contentsOf: URL(fileURLWithPath: "document.pdf"))) ?? Data()
let bytes = Array(data)

do {
    let text = try await extractText(bytes: bytes, mimeType: "application/pdf")
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
