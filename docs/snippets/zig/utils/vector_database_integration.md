```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

// Configure chunking with embeddings — the resulting JSON has a `chunks`
// array where each entry carries `content` and `embedding`. Insert those
// into your vector store (Qdrant, pgvector, Pinecone, etc.) directly from
// the parsed JSON.
pub fn main() !void {
    const config_json =
        \\{
        \\  "chunking": {
        \\    "max_characters": 512,
        \\    "overlap": 50,
        \\    "embedding": {
        \\      "preset": "balanced"
        \\    }
        \\  }
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"document.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{output_json});
}
```
