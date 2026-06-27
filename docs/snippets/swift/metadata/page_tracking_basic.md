```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let configJson = """
{
    "pages": {
        "extract_pages": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

if let pages = result.pages {
    for page in pages {
        let pageContent = page.content().toString()
        print("Page \(page.page_number()):")
        print("  Content: \(pageContent.count) chars")
        print("  Tables: \(page.tables().count)")
        print("  Images: \(page.images().count)")
    }
} else {
    print("No per-page content available")
}
```
