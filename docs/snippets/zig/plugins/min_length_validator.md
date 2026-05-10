```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

const MinLengthValidator = struct {
    min_length: usize,

    pub fn validate(self: *MinLengthValidator, result: [*c]const u8, config: [*c]const u8) !void {
        _ = config;
        const slice = std.mem.sliceTo(result, 0);
        if (slice.len < self.min_length) return error.Validation;
    }

    pub fn should_validate(self: *MinLengthValidator, result: [*c]const u8, config: [*c]const u8) i32 {
        _ = self;
        _ = result;
        _ = config;
        return 1;
    }

    pub fn priority(self: *MinLengthValidator) i32 {
        _ = self;
        return 100;
    }
};

pub fn main() !void {
    var instance = MinLengthValidator{ .min_length = 50 };
    var vtable = kreuzberg.make_validator_vtable(MinLengthValidator, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("min-length-validator");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    _ = kreuzberg.register_validator("min-length-validator", vtable, &instance, &out_error);
}
```
