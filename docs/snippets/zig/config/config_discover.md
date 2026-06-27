```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // The Zig binding accepts JSON config strings. To use a discovered config
    // file, load it from disk into a string and pass it through unchanged.
    const cwd = std.fs.cwd();
    const config_json = cwd.readFileAlloc(allocator, "xberg.json", 1 << 20) catch |err| switch (err) {
        error.FileNotFound => try allocator.dupe(u8, "{}"),
        else => return err,
    };
    defer allocator.free(config_json);

    const input_json = "{\"kind\":\"uri\",\"uri\":\"document.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{output_json});
}
```
