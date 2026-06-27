```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const document_id = "doc_001";

    const config_json =
        \\{
        \\  "chunking": {
        \\    "max_characters": 512,
        \\    "overlap": 50,
        \\    "embedding": {
        \\      "model": {"type": "preset", "name": "balanced"},
        \\      "normalize": true,
        \\      "batch_size": 32
        \\    }
        \\  }
        \\}
    ;

    const result_json = try xberg.extract_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, result_json, .{});
    defer parsed.deinit();

    const root = parsed.value;
    if (root != .object) return;

    const stdout = std.io.getStdOut().writer();

    const chunks_val = root.object.get("chunks") orelse return;
    if (chunks_val != .array) return;

    for (chunks_val.array.items, 0..) |chunk, index| {
        if (chunk != .object) continue;

        const embedding_val = chunk.object.get("embedding") orelse continue;
        if (embedding_val != .array) continue;

        const content_val = chunk.object.get("content") orelse continue;
        if (content_val != .string) continue;

        const record_id = try std.fmt.allocPrint(allocator, "{s}_chunk_{d}", .{ document_id, index });
        defer allocator.free(record_id);

        try stdout.print("id={s} dims={d} content_length={d}\n", .{
            record_id,
            embedding_val.array.items.len,
            content_val.string.len,
        });
        // Persist record_id, content_val.string, and embedding_val.array.items in a vector database.
    }
}
```
