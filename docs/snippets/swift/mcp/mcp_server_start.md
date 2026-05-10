<!-- snippet:syntax-only -->
```swift title="Swift"
import Foundation

// Start the kreuzberg MCP server as a subprocess.
// The Swift bindings do not expose an in-process MCP server; use the
// kreuzberg CLI binary which provides the MCP transport over stdio.
let process = Process()
process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
process.arguments = ["kreuzberg", "mcp"]

try process.run()
process.waitUntilExit()
```
