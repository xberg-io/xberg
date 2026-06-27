```swift title="Swift"
import Xberg

let input = #"{"kind":"uri","uri":"document.pdf"}"#
let output = try await extract(input, "{}")

print("Results: \(output.summary().results())")
```
