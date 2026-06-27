```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "enable_quality_processing": true
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"scanned_document.pdf\"}";
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

    if (root.object.get("quality_score")) |score_val| {
        const score: f64 = switch (score_val) {
            .float => |f| f,
            .integer => |i| @floatFromInt(i),
            else => return,
        };

        if (score < 0.5) {
            try stdout.print("Warning: Low quality extraction ({d:.2})\n", .{score});
        } else {
            try stdout.print("Quality score: {d:.2}\n", .{score});
        }
    }
}
```
