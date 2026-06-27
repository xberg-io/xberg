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

    const input_json = "{\"kind\":\"uri\",\"uri\":\"multilingual_document.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, output_json, .{});
    defer parsed.deinit();

    const output = parsed.value;
    if (output != .object) return;

    const results_val = output.object.get("results") orelse return;
    if (results_val != .array or results_val.array.items.len == 0) return;
    const root = results_val.array.items[0];
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
