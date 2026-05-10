```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

const WordCountProcessor = struct {
    pub fn process(self: *WordCountProcessor, result: [*c]const u8, config: [*c]const u8) !void {
        _ = self;
        _ = result;
        _ = config;
        // The serialized result/config arrive as JSON strings; modify and emit
        // an updated payload through your own pipeline as needed.
    }

    pub fn processing_stage(self: *WordCountProcessor) [*c]const u8 {
        _ = self;
        return "Early";
    }

    pub fn should_process(self: *WordCountProcessor, result: [*c]const u8, config: [*c]const u8) i32 {
        _ = self;
        _ = result;
        _ = config;
        return 1;
    }

    pub fn estimated_duration_ms(self: *WordCountProcessor, result: [*c]const u8) u64 {
        _ = self;
        _ = result;
        return 1;
    }

    pub fn priority(self: *WordCountProcessor) i32 {
        _ = self;
        return 50;
    }
};

pub fn main() !void {
    var instance = WordCountProcessor{};
    var vtable = kreuzberg.make_post_processor_vtable(WordCountProcessor, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("word-count");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    _ = kreuzberg.register_post_processor("word-count", vtable, &instance, &out_error);
}
```
