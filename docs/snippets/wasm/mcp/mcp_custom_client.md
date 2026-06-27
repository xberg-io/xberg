```typescript title="WASM"
// The WASM crate has no MCP client. To integrate with an MCP server,
// drive the xberg CLI from a Node.js host that uses xberg-wasm
// for in-process extraction.
import { spawn } from "node:child_process";
import * as readline from "node:readline";

const mcpProcess = spawn("xberg", ["mcp"]);

const rl = readline.createInterface({
  input: mcpProcess.stdout,
  output: mcpProcess.stdin,
  terminal: false,
});

const request = {
  method: "tools/call",
  params: {
    name: "extract",
    arguments: {
      path: "document.pdf",
      async: true,
    },
  },
};

mcpProcess.stdin.write(`${JSON.stringify(request)}\n`);

rl.on("line", (line) => {
  const response = JSON.parse(line);
  console.log(response);
  mcpProcess.kill();
});

mcpProcess.on("error", (err) => {
  console.error("Failed to start MCP process:", err);
});
```

<!-- snippet:syntax-only --> MCP transport is not exported by the WASM crate; this snippet drives the MCP CLI from the same Node host that loads xberg-wasm.
