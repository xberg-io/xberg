```typescript title="WASM"
// MCP server is provided by the kreuzberg CLI (Rust binary). The WASM build
// targets browser/Node.js extraction and does not embed a server process.
// Spawn the CLI from a Node.js host that consumes the WASM module separately.
import { spawn } from "node:child_process";

const mcpProcess = spawn("kreuzberg", ["mcp"]);

mcpProcess.stdout.on("data", (data) => {
  console.log(`MCP Server: ${data}`);
});

mcpProcess.stderr.on("data", (data) => {
  console.error(`MCP Error: ${data}`);
});

mcpProcess.on("error", (err) => {
  console.error(`Failed to start MCP server: ${err.message}`);
});
```

<!-- snippet:syntax-only --> The MCP server is a CLI feature; the WASM crate does not export an MCP server entry point. This snippet shows how a Node host that uses kreuzberg-wasm for extraction can also drive the standalone MCP CLI.
