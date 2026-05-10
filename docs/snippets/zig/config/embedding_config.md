```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "chunking": {
        \\    "max_characters": 1000,
        \\    "overlap": 200,
        \\    "embedding": {
        \\      "model": {"type": "preset", "name": "balanced"},
        \\      "batch_size": 16,
        \\      "normalize": true,
        \\      "show_download_progress": true
        \\    }
        \\  }
        \\}
    ;

    const result_json = try kreuzberg.extract_file_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const owned = try allocator.dupe(u8, result_json);
    defer allocator.free(owned);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{owned});
}
```
