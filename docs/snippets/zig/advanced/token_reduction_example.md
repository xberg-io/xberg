```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "token_reduction": {
        \\    "mode": "moderate",
        \\    "preserve_important_words": true
        \\  }
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"verbose_document.pdf\"}";
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

    if (root.object.get("original_token_count")) |val| {
        if (val == .integer) {
            try stdout.print("Original tokens: {d}\n", .{val.integer});
        }
    }

    if (root.object.get("reduced_token_count")) |val| {
        if (val == .integer) {
            try stdout.print("Reduced tokens: {d}\n", .{val.integer});
        }
    }
}
```
