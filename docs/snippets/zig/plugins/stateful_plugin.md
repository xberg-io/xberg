```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

const StatefulProcessor = struct {
    call_count: std.atomic.Value(usize) = std.atomic.Value(usize).init(0),

    pub fn process(self: *StatefulProcessor, result: [*c]const u8, config: [*c]const u8) !void {
        _ = result;
        _ = config;
        _ = self.call_count.fetchAdd(1, .acq_rel);
    }

    pub fn processing_stage(self: *StatefulProcessor) [*c]const u8 {
        _ = self;
        return "Middle";
    }

    pub fn should_process(self: *StatefulProcessor, result: [*c]const u8, config: [*c]const u8) i32 {
        _ = self;
        _ = result;
        _ = config;
        return 1;
    }

    pub fn estimated_duration_ms(self: *StatefulProcessor, result: [*c]const u8) u64 {
        _ = self;
        _ = result;
        return 1;
    }

    pub fn priority(self: *StatefulProcessor) i32 {
        _ = self;
        return 50;
    }
};

pub fn main() !void {
    var instance = StatefulProcessor{};
    var vtable = kreuzberg.make_post_processor_vtable(StatefulProcessor, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("stateful-processor");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;
    vtable.shutdown_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_error: ?*?[*c]u8) callconv(.C) i32 {
            _ = out_error;
            const self: *StatefulProcessor = @ptrCast(@alignCast(user_data));
            const count = self.call_count.load(.acquire);
            std.log.info("stateful-processor invoked {d} times", .{count});
            return 0;
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    _ = kreuzberg.register_post_processor("stateful-processor", vtable, &instance, &out_error);
}
```
