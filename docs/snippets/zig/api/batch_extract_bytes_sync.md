```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    // Batch items are passed as a JSON-encoded array across the FFI boundary.
    // `content` is base64-encoded bytes per the FFI schema for BatchBytesItem.
    const items_json =
        \\[
        \\  {"content": "SGVsbG8sIHdvcmxkIQ==", "mime_type": "text/plain", "config": null},
        \\  {"content": "IyBIZWFkaW5nCgpQYXJhZ3JhcGggdGV4dC4=", "mime_type": "text/markdown", "config": null}
        \\]
    ;
    const config_json = "{}";

    const results_json = try kreuzberg.batch_extract_bytes_sync(items_json, config_json);
    defer std.heap.c_allocator.free(results_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{results_json});
}
```
