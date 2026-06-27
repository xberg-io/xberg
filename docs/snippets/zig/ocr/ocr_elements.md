```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "ocr": {
        \\    "backend": "paddleocr",
        \\    "language": "en",
        \\    "element_config": {
        \\      "include_elements": true
        \\    }
        \\  }
        \\}
    ;

    const result_json = try xberg.extract_sync("scanned.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const owned = try allocator.dupe(u8, result_json);
    defer allocator.free(owned);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, owned, .{});
    defer parsed.deinit();

    const stdout = std.io.getStdOut().writer();

    const root = parsed.value;
    if (root != .object) return;

    if (root.object.get("ocr_elements")) |elements_val| {
        if (elements_val == .array) {
            for (elements_val.array.items) |element| {
                if (element != .object) continue;
                if (element.object.get("text")) |text_val| {
                    if (text_val == .string) {
                        try stdout.print("Text: {s}\n", .{text_val.string});
                    }
                }
            }
        }
    }
}
```
