```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // Cloud OCR backends are registered as custom plugins via the Rust core.
    // From Zig, select a registered cloud backend by name through OcrConfig.
    const config_json =
        \\{
        \\  "ocr": {
        \\    "backend": "cloud-ocr",
        \\    "language": "eng"
        \\  }
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"scanned.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const owned = try allocator.dupe(u8, output_json);
    defer allocator.free(owned);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{owned});
}
```
