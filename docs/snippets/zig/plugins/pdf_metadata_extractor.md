```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

const PdfMetadataExtractor = struct {
    processed_count: u64 = 0,

    pub fn process(self: *PdfMetadataExtractor, result: [*c]const u8, config: [*c]const u8) !void {
        _ = result;
        _ = config;
        self.processed_count += 1;
        // Parse the incoming JSON result, append PDF-specific metadata fields,
        // and forward the enriched payload onward.
    }

    pub fn processing_stage(self: *PdfMetadataExtractor) [*c]const u8 {
        _ = self;
        return "Early";
    }

    pub fn should_process(self: *PdfMetadataExtractor, result: [*c]const u8, config: [*c]const u8) i32 {
        _ = self;
        _ = config;
        const slice = std.mem.sliceTo(result, 0);
        return if (std.mem.indexOf(u8, slice, "\"mime_type\":\"application/pdf\"") != null) 1 else 0;
    }

    pub fn estimated_duration_ms(self: *PdfMetadataExtractor, result: [*c]const u8) u64 {
        _ = self;
        _ = result;
        return 2;
    }

    pub fn priority(self: *PdfMetadataExtractor) i32 {
        _ = self;
        return 80;
    }
};

pub fn main() !void {
    var instance = PdfMetadataExtractor{};
    var vtable = kreuzberg.make_post_processor_vtable(PdfMetadataExtractor, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("pdf-metadata-extractor");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    _ = kreuzberg.register_post_processor("pdf-metadata-extractor", vtable, &instance, &out_error);
}
```
