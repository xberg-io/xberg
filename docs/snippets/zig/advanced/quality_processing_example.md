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

    const result_json = try xberg.extract_sync("scanned_document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, result_json, .{});
    defer parsed.deinit();

    const root = parsed.value;
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
