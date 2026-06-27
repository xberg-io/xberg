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

    const result_json = try xberg.extract_sync("verbose_document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, result_json, .{});
    defer parsed.deinit();

    const root = parsed.value;
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
