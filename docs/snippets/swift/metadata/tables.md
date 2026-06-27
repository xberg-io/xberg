```swift title="Swift"
import Foundation
import Xberg
import RustBridge

let config = try extractionConfigFromJson("{}")
let result = try extractSync("document.pdf", nil, config)

let tables = result.tables()
print("Tables: \(tables.count)")

for (index, table) in tables.enumerated() {
    print("Table \(index) on page \(table.page_number())")
    print(table.markdown().toString())

    if let bbox = table.bounding_box() {
        print("  Bounding box: \(bbox.toString())")
    }
}
```
