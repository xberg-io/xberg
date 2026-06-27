```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

// Chunking + embeddings produces RAG-ready output. Each chunk in the
// returned JSON carries `content`, position metadata, and (when an
// embedding preset is configured) an `embedding` vector.
pub fn main() !void {
    const config_json =
        \\{
        \\  "chunking": {
        \\    "max_characters": 500,
        \\    "overlap": 50,
        \\    "embedding": {
        \\      "preset": "balanced"
        \\    }
        \\  }
        \\}
    ;

    const result_json = try xberg.extract_sync("research_paper.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{result_json});
}
```
