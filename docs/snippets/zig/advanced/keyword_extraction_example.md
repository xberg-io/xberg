```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "keywords": {
        \\    "algorithm": "yake",
        \\    "max_keywords": 10,
        \\    "min_score": 0.3,
        \\    "ngram_range": [1, 3]
        \\  }
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"research_paper.pdf\"}";
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

    const keywords_val = root.object.get("extracted_keywords") orelse return;
    if (keywords_val != .array) return;

    for (keywords_val.array.items) |keyword| {
        if (keyword != .object) continue;

        const text_val = keyword.object.get("text") orelse continue;
        const score_val = keyword.object.get("score") orelse continue;
        if (text_val != .string) continue;

        const score: f64 = switch (score_val) {
            .float => |f| f,
            .integer => |i| @floatFromInt(i),
            else => continue,
        };

        try stdout.print("{s}: {d:.4}\n", .{ text_val.string, score });
    }
}
```
