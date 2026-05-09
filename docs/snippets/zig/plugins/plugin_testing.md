```zig title="Zig"
const std = @import("std");
const testing = std.testing;
const kreuzberg = @import("kreuzberg");

const NoopValidator = struct {
    pub fn validate(self: *NoopValidator, result: [*c]const u8, config: [*c]const u8) !void {
        _ = self;
        _ = result;
        _ = config;
    }

    pub fn should_validate(self: *NoopValidator, result: [*c]const u8, config: [*c]const u8) i32 {
        _ = self;
        _ = result;
        _ = config;
        return 1;
    }

    pub fn priority(self: *NoopValidator) i32 {
        _ = self;
        return 50;
    }
};

test "register and unregister validator" {
    var instance = NoopValidator{};
    var vtable = kreuzberg.make_validator_vtable(NoopValidator, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("noop-validator");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    try testing.expectEqual(@as(i32, 0), kreuzberg.register_validator("noop-validator", vtable, &instance, &out_error));

    const validators = try kreuzberg.list_validators();
    defer std.heap.c_allocator.free(validators);
    try testing.expect(std.mem.indexOf(u8, validators, "noop-validator") != null);

    try testing.expectEqual(@as(i32, 0), kreuzberg.unregister_validator("noop-validator", &out_error));
}
```
