<!-- snippet:syntax-only -->

```swift title="Swift"
import Foundation

let process = Process()
process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
process.arguments = ["xberg", "mcp"]

let stdin = Pipe()
let stdout = Pipe()
process.standardInput = stdin
process.standardOutput = stdout

try process.run()

let request: [String: Any] = [
    "method": "tools/call",
    "params": [
        "name": "extract",
        "arguments": [
            "path": "document.pdf",
            "async": true,
        ],
    ],
]

let payload = try JSONSerialization.data(withJSONObject: request)
stdin.fileHandleForWriting.write(payload)
stdin.fileHandleForWriting.write("\n".data(using: .utf8)!)
try stdin.fileHandleForWriting.close()

let data = stdout.fileHandleForReading.availableData
if let line = String(data: data, encoding: .utf8) {
    print(line)
}

process.waitUntilExit()
```
