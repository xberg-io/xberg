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
        \\    "min_score": 0.3,
        \\    "ngram_range": [1, 3],
        \\    "language": "en"
        \\  }
        \\}
    ;

    const result_json = try xberg.extract_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, result_json, .{});
    defer parsed.deinit();
}
```
