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

    const result_json = try xberg.extract_sync("scanned.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
