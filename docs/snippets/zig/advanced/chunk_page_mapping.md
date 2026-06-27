```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "chunking": {
        \\    "max_characters": 500,
        \\    "overlap": 50
        \\  },
        \\  "pages": {
        \\    "extract_pages": true,
        \\    "insert_page_markers": false,
        \\    "marker_format": ""
        \\  }
        \\}
    ;

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

    const chunks_val = root.object.get("chunks") orelse return;
    if (chunks_val != .array) return;

    for (chunks_val.array.items) |chunk| {
        if (chunk != .object) continue;

        const metadata_val = chunk.object.get("metadata") orelse continue;
        if (metadata_val != .object) continue;

        const first_page_val = metadata_val.object.get("first_page") orelse continue;
        const last_page_val = metadata_val.object.get("last_page") orelse continue;
        if (first_page_val != .integer or last_page_val != .integer) continue;

        const first = first_page_val.integer;
        const last = last_page_val.integer;

        if (chunk.object.get("content")) |content_val| {
            if (content_val == .string) {
                const preview_len = @min(50, content_val.string.len);
                if (first == last) {
                    try stdout.print("Chunk: {s}... (Page {d})\n", .{ content_val.string[0..preview_len], first });
                } else {
                    try stdout.print("Chunk: {s}... (Pages {d}-{d})\n", .{ content_val.string[0..preview_len], first, last });
                }
            }
        }
    }
}
```
