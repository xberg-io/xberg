```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

fn extract_text(allocator: std.mem.Allocator, bytes: []const u8, mime_type: []const u8) ![]u8 {
    const config_json = "{}";
    var input_json = std.ArrayList(u8).init(allocator);
    defer input_json.deinit();

    try input_json.writer().writeAll("{\"kind\":\"bytes\",\"bytes\":[");
    for (bytes, 0..) |byte, index| {
        if (index > 0) try input_json.writer().writeAll(",");
        try input_json.writer().print("{d}", .{byte});
    }
    try input_json.writer().print("],\"mime_type\":\"{s}\",\"filename\":\"document.pdf\"}}", .{mime_type});

    return xberg.extract(input_json.items, config_json);
}

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const bytes = std.fs.cwd().readFileAlloc(allocator, "document.pdf", 64 * 1024 * 1024) catch &[_]u8{};
    defer if (bytes.len > 0) allocator.free(bytes);

    const stderr = std.io.getStdErr().writer();
    const output_json = extract_text(allocator, bytes, "application/pdf") catch |err| {
        switch (err) {
            error.UnsupportedFormat => try stderr.print("Format not supported\n", .{}),
            error.Ocr => try stderr.print("OCR failed\n", .{}),
            error.Validation => try stderr.print("Invalid input or configuration\n", .{}),
            else => try stderr.print("Error: {s}\n", .{@errorName(err)}),
        }
        return;
    };
    defer std.heap.c_allocator.free(output_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("Extracted {d} bytes of JSON\n", .{output_json.len});
}
```
