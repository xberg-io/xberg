<!-- snippet:skip -->
```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

// A custom `DocumentExtractor` would implement the Rust trait of the same
// name. The Zig binding does not expose a `make_document_extractor_vtable`
// or `register_document_extractor` entry point — only OCR backends,
// post-processors, validators, and embedding backends are bridged.
//
// To add a custom extractor to the pipeline, implement the trait in a Rust
// shim crate and register it on the Rust side. From Zig, drive extraction
// through `kreuzberg.extract_file_sync` / `kreuzberg.extract_bytes_sync`
// with the registered extractor's MIME type.
pub fn main() !void {
    const config_json = "{}";
    const result_json = try kreuzberg.extract_file_sync("data.json", null, config_json);
    defer std.heap.c_allocator.free(result_json);
}
```
