```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let config = try extractionConfigFromJson("{}")
let result = try extractSync("document.pdf", nil, config)

let metadata = result.metadata()

if let title = metadata.title() {
    print("Title: \(title.toString())")
}
if let subject = metadata.subject() {
    print("Subject: \(subject.toString())")
}
if let language = metadata.language() {
    print("Language: \(language.toString())")
}
if let createdAt = metadata.created_at() {
    print("Created at: \(createdAt.toString())")
}
if let modifiedAt = metadata.modified_at() {
    print("Modified at: \(modifiedAt.toString())")
}
if let createdBy = metadata.created_by() {
    print("Created by: \(createdBy.toString())")
}
if let authors = metadata.authors() {
    let names = authors.map { $0.toString() }
    print("Authors: \(names)")
}
if let keywords = metadata.keywords() {
    let words = keywords.map { $0.toString() }
    print("Keywords: \(words)")
}
if let duration = metadata.extraction_duration_ms() {
    print("Extraction duration (ms): \(duration)")
}
if let pages = metadata.pages() {
    print("Page count: \(pages.total_count())")
}
```
