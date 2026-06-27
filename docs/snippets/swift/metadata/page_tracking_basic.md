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
let result = try extractSync("document.pdf", nil, config)

if let pages = result.pages() {
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
