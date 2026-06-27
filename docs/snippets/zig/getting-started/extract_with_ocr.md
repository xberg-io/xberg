```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();
    _ = allocator;

    const config_json =
        \\{
        \\  "force_ocr": true,
        \\  "ocr": {
        \\    "backend": "tesseract",
        \\    "language": "eng"
        \\  }
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"scanned.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{output_json});
}
```
