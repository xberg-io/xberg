```zig title="Zig"
const std = @import("std");
const kreuzberg = @import("kreuzberg");

const MyEmbedder = struct {
    pub fn dimensions(self: *MyEmbedder) usize {
        _ = self;
        return 768;
    }

    pub fn embed(self: *MyEmbedder, texts: [*c]const u8) ![]u8 {
        _ = self;
        _ = texts;
        // `texts` is a JSON-encoded array of strings. Return a JSON-encoded
        // 2D float array of shape [n_texts, dimensions]; the dispatcher
        // validates the shape on the Rust side.
        return error.Plugin;
    }
};

pub fn main() !void {
    var instance = MyEmbedder{};
    var vtable = kreuzberg.make_embedding_backend_vtable(MyEmbedder, &instance);

    vtable.name_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_name) |ptr| ptr.* = @constCast("my-embedder");
        }
    }.thunk;
    vtable.version_fn = struct {
        fn thunk(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
            _ = user_data;
            if (out_version) |ptr| ptr.* = @constCast("1.0.0");
        }
    }.thunk;

    var out_error: ?[*c]u8 = null;
    _ = kreuzberg.register_embedding_backend("my-embedder", vtable, &instance, &out_error);
}
```
