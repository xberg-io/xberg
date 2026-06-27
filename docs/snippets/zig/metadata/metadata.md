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

    if (root.object.get("metadata")) |metadata_val| {
        if (metadata_val != .object) return;
        const metadata = metadata_val.object;

        if (metadata.get("title")) |title_val| {
            if (title_val == .string) {
                try stdout.print("Title: {s}\n", .{title_val.string});
            }
        }

        if (metadata.get("authors")) |authors_val| {
            if (authors_val == .array) {
                for (authors_val.array.items) |author| {
                    if (author == .string) {
                        try stdout.print("Author: {s}\n", .{author.string});
                    }
                }
            }
        }

        if (metadata.get("language")) |language_val| {
            if (language_val == .string) {
                try stdout.print("Language: {s}\n", .{language_val.string});
            }
        }

        if (metadata.get("created_at")) |created_val| {
            if (created_val == .string) {
                try stdout.print("Created: {s}\n", .{created_val.string});
            }
        }

        if (metadata.get("pages")) |pages_val| {
            if (pages_val == .object) {
                if (pages_val.object.get("total_count")) |total_val| {
                    if (total_val == .integer) {
                        try stdout.print("Pages: {d}\n", .{total_val.integer});
                    }
                }
            }
        }
    }
}
```
