<!-- snippet:syntax-only -->

```zig title="Zig"
const std = @import("std");

// The Zig binding does not expose an MCP client. To talk to the bundled
// `xberg mcp` server, spawn the CLI as a subprocess and exchange
// JSON-RPC messages over stdin/stdout.
pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    var child = std.process.Child.init(&.{ "xberg", "mcp" }, allocator);
    child.stdin_behavior = .Pipe;
    child.stdout_behavior = .Pipe;
    child.stderr_behavior = .Inherit;
    try child.spawn();

    const request =
        \\{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"extract","arguments":{"path":"document.pdf"}}}
        ++ "\n";

    if (child.stdin) |stdin| {
        try stdin.writeAll(request);
        stdin.close();
        child.stdin = null;
    }

    if (child.stdout) |stdout| {
        const response = try stdout.reader().readAllAlloc(allocator, 16 * 1024 * 1024);
        defer allocator.free(response);
        try std.io.getStdOut().writer().print("{s}\n", .{response});
    }

    _ = try child.wait();
}
```
