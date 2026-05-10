<!-- snippet:syntax-only -->
<!-- The Gleam binding does not expose an MCP client API. Connect to a -->
<!-- running `kreuzberg mcp` process by spawning the binary and exchanging -->
<!-- JSON-RPC frames over stdio. The snippet shows the request shape only. -->
```gleam title="Gleam"
import gleam/io

pub fn main() {
  // Example JSON-RPC request body to send on the spawned process's stdin.
  // Encode with `gleam/json` and write to the port; read responses line by line.
  let request =
    "{\"method\":\"tools/call\","
    <> "\"params\":{\"name\":\"extract_file\","
    <> "\"arguments\":{\"path\":\"document.pdf\",\"async\":true}}}"
  io.println(request)
}
```
