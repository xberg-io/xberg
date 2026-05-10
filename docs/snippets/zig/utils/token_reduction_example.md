```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    const config_json =
        \\{
        \\  "token_reduction": {
        \\    "mode": "moderate",
        \\    "preserve_important_words": true
        \\  }
        \\}
    ;

    const result_json = try kreuzberg.extract_file_sync("verbose_document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
