```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "keywords": {
        \\    "algorithm": "yake",
        \\    "max_keywords": 10,
        \\    "min_score": 0.1,
        \\    "ngram_range": [1, 3],
        \\    "language": "en"
        \\  }
        \\}
    ;

    const result_json = try xberg.extract_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const owned = try allocator.dupe(u8, result_json);
    defer allocator.free(owned);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{owned});
}
```
