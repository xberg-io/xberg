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
    const result = results_val.array.items[0];
    if (result != .object) return;
    const root = result.object;
    const content = root.get("content") orelse std.json.Value{ .string = "" };

    const stdout = std.io.getStdOut().writer();
    try stdout.print("Extracted content: {s}\n", .{content.string});

    if (root.get("tables")) |tables_value| {
        const tables = tables_value.array;
        try stdout.print("Tables found: {d}\n", .{tables.items.len});
        for (tables.items, 0..) |table, index| {
            const table_obj = table.object;
            if (table_obj.get("page_number")) |page_number| {
                try stdout.print("  table {d}: page {d}\n", .{ index, page_number.integer });
            } else {
                try stdout.print("  table {d}\n", .{index});
            }
        }
    } else {
        try stdout.print("Tables found: 0\n", .{});
    }
}
```
