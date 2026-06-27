```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "pages": {
        \\    "extract_pages": true
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

    const pages_val = root.object.get("pages") orelse return;
    if (pages_val != .array) return;

    for (pages_val.array.items) |page| {
        if (page != .object) continue;

        const page_number_val = page.object.get("page_number") orelse continue;
        if (page_number_val != .integer) continue;

        try stdout.print("Page {d}:\n", .{page_number_val.integer});

        if (page.object.get("content")) |content_val| {
            if (content_val == .string) {
                try stdout.print("  Content: {d} chars\n", .{content_val.string.len});
            }
        }

        if (page.object.get("tables")) |tables_val| {
            if (tables_val == .array) {
                try stdout.print("  Tables: {d}\n", .{tables_val.array.items.len});
            }
        }

        if (page.object.get("images")) |images_val| {
            if (images_val == .array) {
                try stdout.print("  Images: {d}\n", .{images_val.array.items.len});
            }
        }
    }
}
```
