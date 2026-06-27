```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json = "{}";
    const result_json = try xberg.extract_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, result_json, .{});
    defer parsed.deinit();

    const root = parsed.value;
    if (root != .object) return;

    const stdout = std.io.getStdOut().writer();

    const tables_val = root.object.get("tables") orelse return;
    if (tables_val != .array) return;

    for (tables_val.array.items) |table| {
        if (table != .object) continue;

        if (table.object.get("cells")) |cells_val| {
            if (cells_val == .array) {
                try stdout.print("Table with {d} rows\n", .{cells_val.array.items.len});

                for (cells_val.array.items) |row_val| {
                    if (row_val != .array) continue;
                    try stdout.print("  Row:", .{});
                    for (row_val.array.items) |cell_val| {
                        if (cell_val == .string) {
                            try stdout.print(" [{s}]", .{cell_val.string});
                        }
                    }
                    try stdout.print("\n", .{});
                }
            }
        }

        if (table.object.get("markdown")) |markdown_val| {
            if (markdown_val == .string) {
                try stdout.print("{s}\n", .{markdown_val.string});
            }
        }

        if (table.object.get("page_number")) |page_val| {
            if (page_val == .integer) {
                try stdout.print("Page: {d}\n", .{page_val.integer});
            }
        }
    }
}
```
