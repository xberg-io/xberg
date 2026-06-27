```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "language_detection": {
        \\    "enabled": true,
        \\    "min_confidence": 0.8,
        \\    "detect_multiple": true
        \\  }
        \\}
    ;

    const result_json = try xberg.extract_sync("multilingual_document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, result_json, .{});
    defer parsed.deinit();

    const root = parsed.value;
    if (root != .object) return;

    const stdout = std.io.getStdOut().writer();

    try stdout.print("Detected languages:", .{});
    if (root.object.get("detected_languages")) |languages_val| {
        if (languages_val == .array) {
            for (languages_val.array.items) |lang_val| {
                if (lang_val == .string) {
                    try stdout.print(" {s}", .{lang_val.string});
                }
            }
        }
    }
    try stdout.print("\n", .{});
}
```
