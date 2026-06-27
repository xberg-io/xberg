```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    const config_json =
        \\{
        \\  "keywords": {
        \\    "algorithm": "yake",
        \\    "max_keywords": 10,
        \\    "min_score": 0.3
        \\  }
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"research_paper.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{output_json});
}
```
