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

    const input_json = "{\"kind\":\"uri\",\"uri\":\"scanned.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const owned = try allocator.dupe(u8, output_json);
    defer allocator.free(owned);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, owned, .{});
    defer parsed.deinit();

    const stdout = std.io.getStdOut().writer();

    const output = parsed.value;
    if (output != .object) return;

    const results_val = output.object.get("results") orelse return;
    if (results_val != .array or results_val.array.items.len == 0) return;
    const root = results_val.array.items[0];
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
