```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let config = try extractionConfigFromJson("{}")
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

let content = result.content.toString()
let utf8 = Array(content.utf8)

guard let pageStructure = result.metadata.pages() else {
    print("No page structure available")
    exit(0)
}
guard let boundaries = pageStructure.boundaries() else {
    print("No page boundaries available")
    exit(0)
}

for (index, boundary) in boundaries.enumerated() {
    if index >= 3 { break }

    let byteStart = boundary.byte_start()
    let byteEnd = boundary.byte_end()
    let pageBytes = Array(utf8[byteStart..<byteEnd])
    let pageText = String(bytes: pageBytes, encoding: .utf8) ?? ""
    let previewEnd = min(100, pageText.count)
    let preview = String(pageText.prefix(previewEnd))

    print("Page \(boundary.page_number()):")
    print("  Byte range: \(byteStart)-\(byteEnd)")
    print("  Preview: \(preview)...")
}
```
