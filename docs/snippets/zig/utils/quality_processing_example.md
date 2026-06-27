```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    const config_json =
        \\{
        \\  "enable_quality_processing": true
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"scanned_document.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{output_json});
}
```
