```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const content = try std.fs.cwd().readFileAlloc(allocator, "document.pdf", 64 * 1024 * 1024);
    defer allocator.free(content);

    const config_json = "{}";
    const result_json = try kreuzberg.extract_bytes_sync(content, "application/pdf", config_json);
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
