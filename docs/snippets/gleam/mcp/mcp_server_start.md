<!-- snippet:syntax-only -->
<!-- The Gleam binding does not expose an MCP server API. The MCP server -->
<!-- is provided by the `kreuzberg` CLI binary; start it as an external -->
<!-- process and communicate over stdio. -->
```gleam title="Gleam"
import gleam/erlang/os
import gleam/io

pub fn main() {
  // Spawn `kreuzberg mcp` as an external process from your Gleam app.
  // Replace this with your preferred port/command runner; the snippet
  // illustrates the entrypoint, not a full client implementation.
  let _ = os.get_env("PATH")
  io.println("Run: kreuzberg mcp")
  io.println("Then connect to the process via JSON-RPC over stdio.")
}
```
