```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "pdf_options": {
        \\    "extract_images": true,
        \\    "passwords": ["password123"],
        \\    "extract_metadata": true,
        \\    "extract_annotations": true
        \\  }
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"encrypted.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const owned = try allocator.dupe(u8, output_json);
    defer allocator.free(owned);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{owned});
}
```
