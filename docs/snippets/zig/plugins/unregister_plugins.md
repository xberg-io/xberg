```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    var out_error: ?[*c]u8 = null;

    _ = kreuzberg.unregister_post_processor("word-count", &out_error);
    _ = kreuzberg.unregister_validator("min-length-validator", &out_error);
    _ = kreuzberg.unregister_ocr_backend("custom-ocr", &out_error);
    _ = kreuzberg.unregister_embedding_backend("my-embedder", &out_error);
}
```
