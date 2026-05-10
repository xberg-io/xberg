```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

const PdfOnlyProcessor = struct {
    pub fn process(self: *PdfOnlyProcessor, result: [*c]const u8, config: [*c]const u8) !void {
        _ = self;
        _ = result;
        _ = config;
        // PDF-specific transforms go here. Parse the JSON result, mutate
        // metadata/content, and forward through the pipeline.
    }

    pub fn processing_stage(self: *PdfOnlyProcessor) [*c]const u8 {
        _ = self;
        return "Middle";
    }

    pub fn should_process(self: *PdfOnlyProcessor, result: [*c]const u8, config: [*c]const u8) i32 {
        _ = self;
        _ = config;
        const slice = std.mem.sliceTo(result, 0);
        return if (std.mem.indexOf(u8, slice, "\"mime_type\":\"application/pdf\"") != null) 1 else 0;
    }

    pub fn estimated_duration_ms(self: *PdfOnlyProcessor, result: [*c]const u8) u64 {
        _ = self;
        _ = result;
        return 5;
    }

    pub fn priority(self: *PdfOnlyProcessor) i32 {
        _ = self;
        return 70;
    }
};

pub fn main() !void {
    var instance = PdfOnlyProcessor{};
    var vtable = kreuzberg.make_post_processor_vtable(PdfOnlyProcessor, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("pdf-only-processor");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    _ = kreuzberg.register_post_processor("pdf-only-processor", vtable, &instance, &out_error);
}
```
