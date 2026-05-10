```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();
    _ = allocator;

    const stdout = std.io.getStdOut().writer();
    try stdout.print("Hello from kreuzberg-zig\n", .{});

    const config_json = "{}";
    const result_json = try kreuzberg.extract_file_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    try stdout.print("{s}\n", .{result_json});
}
```
