```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

const QualityScoreValidator = struct {
    threshold: f64,

    pub fn validate(self: *QualityScoreValidator, result: [*c]const u8, config: [*c]const u8) !void {
        _ = self;
        _ = result;
        _ = config;
        // Parse `result` JSON, look up `metadata.additional.quality_score`,
        // and return error.Validation if it falls below `self.threshold`.
    }

    pub fn should_validate(self: *QualityScoreValidator, result: [*c]const u8, config: [*c]const u8) i32 {
        _ = self;
        _ = result;
        _ = config;
        return 1;
    }

    pub fn priority(self: *QualityScoreValidator) i32 {
        _ = self;
        return 75;
    }
};

pub fn main() !void {
    var instance = QualityScoreValidator{ .threshold = 0.5 };
    var vtable = kreuzberg.make_validator_vtable(QualityScoreValidator, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("quality-score-validator");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    _ = kreuzberg.register_validator("quality-score-validator", vtable, &instance, &out_error);
}
```
