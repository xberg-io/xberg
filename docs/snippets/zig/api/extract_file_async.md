<!-- snippet:syntax-only -->
```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

// Note: the Zig binding is sync-only. There is no `extract_file` async variant —
// the FFI surface exposes blocking entry points that internally drive the global
// Tokio runtime. Use `extract_file_sync` from any thread.
pub fn main() !void {
    const config_json = "{}";
    const result_json = try kreuzberg.extract_file_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
