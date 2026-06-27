```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

// Mirrors XbergDocumentExtractorVTable from the C FFI.
const DocumentExtractorVTable = extern struct {
    name_fn: ?*const fn (user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void,
    version_fn: ?*const fn (user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void,
    initialize_fn: ?*const fn (user_data: ?*anyopaque, out_error: ?*?[*c]u8) callconv(.C) i32,
    shutdown_fn: ?*const fn (user_data: ?*anyopaque, out_error: ?*?[*c]u8) callconv(.C) i32,
    extract: ?*const fn (
        user_data: ?*anyopaque,
        content: [*c]const u8,
        content_len: usize,
        mime_type: [*c]const u8,
        config: [*c]const u8,
        out_result: ?*?[*c]u8,
        out_error: ?*?[*c]u8,
    ) callconv(.C) i32,
    extract: ?*const fn (
        user_data: ?*anyopaque,
        path: [*c]const u8,
        mime_type: [*c]const u8,
        config: [*c]const u8,
        out_result: ?*?[*c]u8,
        out_error: ?*?[*c]u8,
    ) callconv(.C) i32,
    supported_mime_types: ?*const fn (user_data: ?*anyopaque, out_result: ?*?[*c]u8) callconv(.C) i32,
    priority: ?*const fn (user_data: ?*anyopaque) callconv(.C) i32,
    can_handle: ?*const fn (
        user_data: ?*anyopaque,
        path: [*c]const u8,
        mime_type: [*c]const u8,
    ) callconv(.C) i32,
    as_sync_extractor: ?*const fn (user_data: ?*anyopaque) callconv(.C) i32,
    free_user_data: ?*const fn (user_data: ?*anyopaque) callconv(.C) void,
};

// Simple state struct for the extractor instance.
const SimpleExtractorState = struct {
    source_format: [:0]const u8,
    supported_mimes: [:0]const u8,
};

extern "xberg_ffi" fn xberg_register_document_extractor(
    name: [*c]const u8,
    vtable: DocumentExtractorVTable,
    user_data: ?*anyopaque,
    out_error: ?*?[*c]u8,
) i32;

extern "xberg_ffi" fn xberg_free_string(ptr: [*c]u8) void;

// Callbacks for the custom extractor.
fn extract_impl(
    user_data: ?*anyopaque,
    content: [*c]const u8,
    content_len: usize,
    _: [*c]const u8,
    _: [*c]const u8,
    out_result: ?*?[*c]u8,
    out_error: ?*?[*c]u8,
) callconv(.C) i32 {
    const state: *SimpleExtractorState = @ptrCast(@alignCast(user_data));
    _ = state;

    // Minimal extraction: wrap content in JSON.
    var arena = std.heap.ArenaAllocator.init(std.heap.c_allocator);
    defer arena.deinit();
    const allocator = arena.allocator();

    const content_slice = content[0..content_len];
    const result = std.fmt.allocPrint(
        allocator,
        "{{\"content\": \"{s}\", \"mime_type\": \"application/octet-stream\"}}",
        .{content_slice},
    ) catch {
        if (out_error) |ptr| {
            const err = "OOM during extraction";
            if (std.heap.c_allocator.allocSentinel(u8, err.len, 0)) |cstr| {
                @memcpy(cstr[0..err.len], err);
                ptr.* = cstr.ptr;
            }
        }
        return 1;
    };

    const result_cstr = std.heap.c_allocator.allocSentinel(u8, result.len, 0) catch {
        if (out_error) |ptr| {
            const err = "OOM allocating result";
            if (std.heap.c_allocator.allocSentinel(u8, err.len, 0)) |cstr| {
                @memcpy(cstr[0..err.len], err);
                ptr.* = cstr.ptr;
            }
        }
        return 1;
    };
    @memcpy(result_cstr[0..result.len], result);

    if (out_result) |ptr| ptr.* = result_cstr.ptr;
    return 0;
}

fn supported_mimes_impl(user_data: ?*anyopaque, out_result: ?*?[*c]u8) callconv(.C) i32 {
    const state: *SimpleExtractorState = @ptrCast(@alignCast(user_data));
    const mimes = state.supported_mimes;
    const mimes_cstr = std.heap.c_allocator.allocSentinel(u8, mimes.len, 0) catch return 1;
    @memcpy(mimes_cstr[0..mimes.len], mimes);
    if (out_result) |ptr| ptr.* = mimes_cstr.ptr;
    return 0;
}

fn name_impl(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
    _ = user_data;
    const name = "zig-simple-extractor";
    if (std.heap.c_allocator.allocSentinel(u8, name.len, 0)) |cstr| {
        @memcpy(cstr[0..name.len], name);
        if (out_name) |ptr| ptr.* = cstr.ptr;
    }
}

fn version_impl(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
    _ = user_data;
    const version = "0.1.0";
    if (std.heap.c_allocator.allocSentinel(u8, version.len, 0)) |cstr| {
        @memcpy(cstr[0..version.len], version);
        if (out_version) |ptr| ptr.* = cstr.ptr;
    }
}

fn init_impl(user_data: ?*anyopaque, out_error: ?*?[*c]u8) callconv(.C) i32 {
    _ = user_data;
    _ = out_error;
    return 0;
}

fn shutdown_impl(user_data: ?*anyopaque, out_error: ?*?[*c]u8) callconv(.C) i32 {
    _ = user_data;
    _ = out_error;
    return 0;
}

fn priority_impl(user_data: ?*anyopaque) callconv(.C) i32 {
    _ = user_data;
    return 60; // Higher than default
}

fn cleanup_state(user_data: ?*anyopaque) callconv(.C) void {
    const state: *SimpleExtractorState = @ptrCast(@alignCast(user_data));
    std.heap.c_allocator.free(state.supported_mimes);
    std.heap.c_allocator.destroy(state);
}

pub fn main() !void {
    // Create extractor state on the heap.
    const state = try std.heap.c_allocator.create(SimpleExtractorState);
    state.source_format = try std.heap.c_allocator.dupeZ(u8, "custom");
    state.supported_mimes = try std.heap.c_allocator.dupeZ(u8, "[\"application/octet-stream\"]");

    var out_error: ?[*c]u8 = null;
    defer if (out_error) |ptr| xberg_free_string(ptr);

    // Build and register the vtable.
    const vtable = DocumentExtractorVTable{
        .name_fn = name_impl,
        .version_fn = version_impl,
        .initialize_fn = init_impl,
        .shutdown_fn = shutdown_impl,
        .extract = extract_impl,
        .extract = null,
        .supported_mime_types = supported_mimes_impl,
        .priority = priority_impl,
        .can_handle = null,
        .as_sync_extractor = null,
        .free_user_data = cleanup_state,
    };

    const rc = xberg_register_document_extractor(
        "zig-simple-extractor",
        vtable,
        state,
        &out_error,
    );

    const stdout = std.io.getStdOut().writer();
    if (rc == 0) {
        try stdout.print("Registered zig-simple-extractor with custom state\n", .{});
    } else {
        if (out_error) |err_ptr| {
            const err_msg = std.mem.sliceTo(err_ptr, 0);
            try stdout.print("Registration failed: {s}\n", .{err_msg});
        } else {
            try stdout.print("Registration failed (no error message)\n", .{});
        }
    }
}
```
