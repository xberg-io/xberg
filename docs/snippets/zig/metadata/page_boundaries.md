```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json = "{}";
    const input_json = "{\"kind\":\"uri\",\"uri\":\"document.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, output_json, .{});
    defer parsed.deinit();

    const output = parsed.value;
    if (output != .object) return;

    const results_val = output.object.get("results") orelse return;
    if (results_val != .array or results_val.array.items.len == 0) return;
    const root = results_val.array.items[0];
    if (root != .object) return;

    const stdout = std.io.getStdOut().writer();

    const content_val = root.object.get("content") orelse return;
    if (content_val != .string) return;
    const content = content_val.string;

    const metadata_val = root.object.get("metadata") orelse return;
    if (metadata_val != .object) return;

    const pages_val = metadata_val.object.get("pages") orelse return;
    if (pages_val != .object) return;

    const boundaries_val = pages_val.object.get("boundaries") orelse return;
    if (boundaries_val != .array) return;

    var index: usize = 0;
    for (boundaries_val.array.items) |boundary| {
        if (index >= 3) break;
        if (boundary != .object) continue;

        const byte_start_val = boundary.object.get("byte_start") orelse continue;
        const byte_end_val = boundary.object.get("byte_end") orelse continue;
        const page_number_val = boundary.object.get("page_number") orelse continue;

        if (byte_start_val != .integer or byte_end_val != .integer or page_number_val != .integer) {
            continue;
        }

        const byte_start: usize = @intCast(byte_start_val.integer);
        const byte_end: usize = @intCast(byte_end_val.integer);
        const page_number = page_number_val.integer;

        const page_text = content[byte_start..byte_end];
        const preview_end = @min(@as(usize, 100), page_text.len);

        try stdout.print("Page {d}:\n", .{page_number});
        try stdout.print("  Byte range: {d}-{d}\n", .{ byte_start, byte_end });
        try stdout.print("  Preview: {s}...\n", .{page_text[0..preview_end]});

        index += 1;
    }
}
```
