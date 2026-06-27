```swift title="Swift"
import Xberg

let inputs = [
    try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#),
    try extractInputFromJson(
        #"{"kind":"bytes","bytes":[72,101,108,108,111],"mime_type":"text/plain","filename":"note.txt"}"#
    ),
]

let output = try await extractBatch(inputs, "{}")
print("Results: \(output.summary().results())")
```
