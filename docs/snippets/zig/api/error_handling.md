```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    const config_json = "{}";
    const result_json = xberg.extract_sync("document.pdf", null, config_json) catch |err| {
        const stderr = std.io.getStdErr().writer();
        switch (err) {
            error.Io => try stderr.print("File error\n", .{}),
            error.UnsupportedFormat => try stderr.print("Unsupported format\n", .{}),
            error.Parsing => try stderr.print("Corrupt or invalid document\n", .{}),
            error.MissingDependency => try stderr.print("Missing dependency — install required backend\n", .{}),
            error.Ocr => try stderr.print("OCR processing failed\n", .{}),
            error.OutOfMemory => try stderr.print("Out of memory\n", .{}),
            else => try stderr.print("Extraction failed: {s}\n", .{@errorName(err)}),
        }
        if (xberg._last_error()) |context| {
            try stderr.print("  context: {s}\n", .{context});
        }
        return;
    };
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
