```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    try kreuzberg.clear_ocr_backends();
    try kreuzberg.clear_post_processors();
    try kreuzberg.clear_validators();

    const stdout = std.io.getStdOut().writer();
    try stdout.print("All plugins cleared\n", .{});
}
```
