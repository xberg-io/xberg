```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json = "{}";
    const result_json = try kreuzberg.extract_file_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, result_json, .{});
    defer parsed.deinit();

    const root = parsed.value.object;
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
