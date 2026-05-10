```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

// `embed_texts` takes JSON-encoded inputs across the FFI boundary:
// - `texts`: a JSON array of strings
// - `config`: a JSON-encoded `EmbeddingConfig`
// It returns a JSON-encoded 2D float array (one row per input text).
pub fn main() !void {
    const texts_json =
        \\["Hello, world!", "Kreuzberg is fast"]
    ;
    const config_json =
        \\{
        \\  "model": {"type": "preset", "name": "balanced"},
        \\  "normalize": true
        \\}
    ;

    const embeddings_json = try kreuzberg.embed_texts(texts_json, config_json);
    defer std.heap.c_allocator.free(embeddings_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{embeddings_json});
}
```
