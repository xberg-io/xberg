```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

const LoggingProcessor = struct {
    pub fn process(self: *LoggingProcessor, result: [*c]const u8, config: [*c]const u8) !void {
        _ = self;
        _ = config;
        const slice = std.mem.sliceTo(result, 0);
        std.log.info("post-processor invoked, payload bytes={d}", .{slice.len});
    }

    pub fn processing_stage(self: *LoggingProcessor) [*c]const u8 {
        _ = self;
        return "Late";
    }

    pub fn should_process(self: *LoggingProcessor, result: [*c]const u8, config: [*c]const u8) i32 {
        _ = self;
        _ = result;
        _ = config;
        return 1;
    }

    pub fn estimated_duration_ms(self: *LoggingProcessor, result: [*c]const u8) u64 {
        _ = self;
        _ = result;
        return 0;
    }

    pub fn priority(self: *LoggingProcessor) i32 {
        _ = self;
        return 10;
    }
};

pub fn main() !void {
    var instance = LoggingProcessor{};
    var vtable = kreuzberg.make_post_processor_vtable(LoggingProcessor, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("logging-processor");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;
    vtable.initialize_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_error: ?*?[*c]u8) callconv(.C) i32 {
            _ = user_data;
            _ = out_error;
            std.log.info("logging-processor initialised", .{});
            return 0;
        }
    }.thunk;
    vtable.shutdown_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_error: ?*?[*c]u8) callconv(.C) i32 {
            _ = user_data;
            _ = out_error;
            std.log.info("logging-processor shut down", .{});
            return 0;
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    _ = kreuzberg.register_post_processor("logging-processor", vtable, &instance, &out_error);
}
```
