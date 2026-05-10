```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

const ContentValidator = struct {
    pub fn validate(self: *ContentValidator, result: [*c]const u8, config: [*c]const u8) !void {
        _ = self;
        _ = config;
        const slice = std.mem.sliceTo(result, 0);
        if (slice.len == 0) return error.Validation;
    }

    pub fn should_validate(self: *ContentValidator, result: [*c]const u8, config: [*c]const u8) i32 {
        _ = self;
        _ = result;
        _ = config;
        return 1;
    }

    pub fn priority(self: *ContentValidator) i32 {
        _ = self;
        return 50;
    }
};

pub fn main() !void {
    var instance = ContentValidator{};
    var vtable = kreuzberg.make_validator_vtable(ContentValidator, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("content-validator");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    _ = kreuzberg.register_validator("content-validator", vtable, &instance, &out_error);
}
```
