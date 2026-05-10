```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

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

    const result_json = try kreuzberg.extract_file_sync("scanned.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
