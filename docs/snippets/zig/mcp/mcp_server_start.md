<!-- snippet:syntax-only -->
```zig title="Zig"
const std = @import("std");

// The Zig binding does not expose the MCP server programmatically. Launch
// the bundled `kreuzberg mcp` CLI as a subprocess to start the server.
pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    var child = std.process.Child.init(&.{ "kreuzberg", "mcp" }, allocator);
    child.stdin_behavior = .Inherit;
    child.stdout_behavior = .Inherit;
    child.stderr_behavior = .Inherit;
    try child.spawn();

    const term = try child.wait();
    try std.io.getStdOut().writer().print("kreuzberg mcp exited: {any}\n", .{term});
}
```
