```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // Build the config JSON programmatically with std.json.
    var arena = std.heap.ArenaAllocator.init(allocator);
    defer arena.deinit();
    const a = arena.allocator();

    var root = std.json.ObjectMap.init(a);
    try root.put("use_cache", std.json.Value{ .bool = true });
    try root.put("enable_quality_processing", std.json.Value{ .bool = true });

    var ocr = std.json.ObjectMap.init(a);
    try ocr.put("backend", std.json.Value{ .string = "tesseract" });
    try ocr.put("language", std.json.Value{ .string = "eng+deu" });
    try root.put("ocr", std.json.Value{ .object = ocr });

    var chunking = std.json.ObjectMap.init(a);
    try chunking.put("max_characters", std.json.Value{ .integer = 1000 });
    try chunking.put("overlap", std.json.Value{ .integer = 200 });
    try root.put("chunking", std.json.Value{ .object = chunking });

    const config_value = std.json.Value{ .object = root };
    var buffer = std.ArrayList(u8).init(a);
    try std.json.stringify(config_value, .{}, buffer.writer());

    const result_json = try xberg.extract_sync("document.pdf", null, buffer.items);
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
