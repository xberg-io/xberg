```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    const ocr_backends = try kreuzberg.list_ocr_backends();
    defer std.heap.c_allocator.free(ocr_backends);

    const post_processors = try kreuzberg.list_post_processors();
    defer std.heap.c_allocator.free(post_processors);

    const validators = try kreuzberg.list_validators();
    defer std.heap.c_allocator.free(validators);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("OCR backends: {s}\n", .{ocr_backends});
    try stdout.print("Post-processors: {s}\n", .{post_processors});
    try stdout.print("Validators: {s}\n", .{validators});
}
```
