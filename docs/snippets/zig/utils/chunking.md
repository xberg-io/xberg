```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    const config_json =
        \\{
        \\  "chunking": {
        \\    "max_characters": 1500,
        \\    "overlap": 200
        \\  }
        \\}
    ;

    const result_json = try kreuzberg.extract_file_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
