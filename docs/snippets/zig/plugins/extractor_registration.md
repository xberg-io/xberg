```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

// VTable struct for DocumentExtractor; mirrors XbergDocumentExtractorVTable.
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

extern "xberg_ffi" fn xberg_register_document_extractor(
    name: [*c]const u8,
    vtable: DocumentExtractorVTable,
    user_data: ?*anyopaque,
    out_error: ?*?[*c]u8,
) i32;

extern "xberg_ffi" fn xberg_unregister_document_extractor(
    name: [*c]const u8,
    out_error: ?*?[*c]u8,
) i32;

extern "xberg_ffi" fn xberg_free_string(ptr: [*c]u8) void;

// Implement callback functions for the extractor.
fn extract_fn(
    user_data: ?*anyopaque,
    content: [*c]const u8,
    content_len: usize,
    mime_type: [*c]const u8,
    config: [*c]const u8,
    out_result: ?*?[*c]u8,
    out_error: ?*?[*c]u8,
) callconv(.C) i32 {
    _ = user_data;
    _ = content;
    _ = content_len;
    _ = config;

    const mime_str = std.mem.sliceTo(mime_type, 0);
    if (std.mem.eql(u8, mime_str, "application/json")) {
        const result = "{\"content\": \"Extracted from JSON\"}";
        const result_cstr = std.heap.c_allocator.allocSentinel(u8, result.len, 0) catch return 1;
        @memcpy(result_cstr[0..result.len], result);
        if (out_result) |ptr| ptr.* = result_cstr.ptr;
        return 0;
    }
    if (out_error) |ptr| {
        const err_msg = "Unsupported MIME type";
        const err_cstr = std.heap.c_allocator.allocSentinel(u8, err_msg.len, 0) catch return 1;
        @memcpy(err_cstr[0..err_msg.len], err_msg);
        ptr.* = err_cstr.ptr;
    }
    return 1;
}

fn supported_mime_types_fn(user_data: ?*anyopaque, out_result: ?*?[*c]u8) callconv(.C) i32 {
    _ = user_data;
    const mime_types = "[\"application/json\"]";
    const cstr = std.heap.c_allocator.allocSentinel(u8, mime_types.len, 0) catch return 1;
    @memcpy(cstr[0..mime_types.len], mime_types);
    if (out_result) |ptr| ptr.* = cstr.ptr;
    return 0;
}

fn name_fn(user_data: ?*anyopaque, out_name: ?*?[*c]u8) callconv(.C) void {
    _ = user_data;
    const name = "zig-json-extractor";
    if (std.heap.c_allocator.allocSentinel(u8, name.len, 0)) |cstr| {
        @memcpy(cstr[0..name.len], name);
        if (out_name) |ptr| ptr.* = cstr.ptr;
    }
}

fn version_fn(user_data: ?*anyopaque, out_version: ?*?[*c]u8) callconv(.C) void {
    _ = user_data;
    const version = "0.1.0";
    if (std.heap.c_allocator.allocSentinel(u8, version.len, 0)) |cstr| {
        @memcpy(cstr[0..version.len], version);
        if (out_version) |ptr| ptr.* = cstr.ptr;
    }
}

fn initialize_fn(user_data: ?*anyopaque, out_error: ?*?[*c]u8) callconv(.C) i32 {
    _ = user_data;
    _ = out_error;
    return 0; // Success
}

fn shutdown_fn(user_data: ?*anyopaque, out_error: ?*?[*c]u8) callconv(.C) i32 {
    _ = user_data;
    _ = out_error;
    return 0; // Success
}

fn priority_fn(user_data: ?*anyopaque) callconv(.C) i32 {
    _ = user_data;
    return 50; // Default priority
}

pub fn main() !void {
    var out_error: ?[*c]u8 = null;
    defer if (out_error) |ptr| xberg_free_string(ptr);

    // Build the vtable.
    const vtable = DocumentExtractorVTable{
        .name_fn = name_fn,
        .version_fn = version_fn,
        .initialize_fn = initialize_fn,
        .shutdown_fn = shutdown_fn,
        .extract = extract_fn,
        .extract = null,
        .supported_mime_types = supported_mime_types_fn,
        .priority = priority_fn,
        .can_handle = null,
        .as_sync_extractor = null,
        .free_user_data = null,
    };

    // Register the extractor with null user_data (no state).
    const register_rc = xberg_register_document_extractor(
        "zig-json-extractor",
        vtable,
        null,
        &out_error,
    );

    if (register_rc != 0) {
        const stdout = std.io.getStdOut().writer();
        if (out_error) |err_ptr| {
            const err_msg = std.mem.sliceTo(err_ptr, 0);
            try stdout.print("Registration failed: {s}\n", .{err_msg});
        }
        return;
    }

    const stdout = std.io.getStdOut().writer();
    try stdout.print("Successfully registered zig-json-extractor\n", .{});

    // Unregister the extractor when done.
    out_error = null;
    const unregister_rc = xberg_unregister_document_extractor("zig-json-extractor", &out_error);
    if (unregister_rc == 0) {
        try stdout.print("Successfully unregistered zig-json-extractor\n", .{});
    }
}
```
