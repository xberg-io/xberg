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

    const result_json = try xberg.extract_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
