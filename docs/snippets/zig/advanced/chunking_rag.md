```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "chunking": {
        \\    "max_characters": 500,
        \\    "overlap": 50,
        \\    "embedding": {
        \\      "model": {"type": "preset", "name": "balanced"},
        \\      "normalize": true
        \\    }
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

    const chunks_val = root.object.get("chunks") orelse return;
    if (chunks_val != .array) return;

    for (chunks_val.array.items, 0..) |chunk, index| {
        if (chunk != .object) continue;

        if (chunk.object.get("content")) |content_val| {
            if (content_val == .string) {
                const preview_len = @min(100, content_val.string.len);
                try stdout.print("Chunk {d}: {s}...\n", .{ index, content_val.string[0..preview_len] });
            }
        }

        if (chunk.object.get("embedding")) |embedding_val| {
            if (embedding_val == .array) {
                try stdout.print("  Embedding: {d} dimensions\n", .{embedding_val.array.items.len});
            }
        }
    }
}
```
