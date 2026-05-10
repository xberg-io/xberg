```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

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

    const result_json = try kreuzberg.extract_file_sync("scanned.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const owned = try allocator.dupe(u8, result_json);
    defer allocator.free(owned);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{owned});
}
```
