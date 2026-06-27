```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "chunking": {
        \\    "max_characters": 1000,
        \\    "overlap": 200,
        \\    "chunker_type": "markdown",
        \\    "prepend_heading_context": true
        \\  }
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"document.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const owned = try allocator.dupe(u8, output_json);
    defer allocator.free(owned);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{owned});
}
```
