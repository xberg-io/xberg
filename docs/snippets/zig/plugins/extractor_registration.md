<!-- snippet:skip -->
```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

// Custom document extractors are implemented against the Rust
// `DocumentExtractor` trait. The Zig binding does not bridge that trait —
// only `OcrBackend`, `PostProcessor`, `Validator`, and `EmbeddingBackend`
// vtables are available via `make_*_vtable` / `register_*`.
//
// To inspect the document extractors already registered in the Rust core,
// call `list_document_extractors`:
pub fn main() !void {
    const extractors = try kreuzberg.list_document_extractors();
    defer std.heap.c_allocator.free(extractors);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("Document extractors: {s}\n", .{extractors});
}
```
