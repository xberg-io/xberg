```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    // Batch items are passed as a JSON-encoded array across the FFI boundary.
    const items_json =
        \\[
        \\  {"path": "doc1.pdf", "config": null},
        \\  {"path": "doc2.docx", "config": null},
        \\  {"path": "report.pdf", "config": null}
        \\]
    ;
    const config_json = "{}";

    const results_json = try kreuzberg.batch_extract_files_sync(items_json, config_json);
    defer std.heap.c_allocator.free(results_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{results_json});
}
```
