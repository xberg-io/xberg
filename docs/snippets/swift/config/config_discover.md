```swift title="Swift"
import Foundation
import Xberg
import RustBridge

// Swift bindings build configs via JSON. To honor an on-disk
// `xberg.{toml,yaml,json}`, load the file and pass its JSON
// representation to `extractionConfigFromJson`. Unknown formats
// can be normalized to JSON on the caller side.
let configJson: String
if let data = try? Data(contentsOf: URL(fileURLWithPath: "xberg.json")),
   let text = String(data: data, encoding: .utf8) {
    configJson = text
} else {
    configJson = "{}"
}

let config = try extractionConfigFromJson(configJson)
let result = try extractSync("document.pdf", nil, config)
print(result.content().toString())
```
