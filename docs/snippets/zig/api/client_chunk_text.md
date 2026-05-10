```zig title="Zig"
const std = @import("std");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const file_bytes = try std.fs.cwd().readFileAlloc(allocator, "document.pdf", 64 * 1024 * 1024);
    defer allocator.free(file_bytes);

    const boundary = "----kreuzberg-zig-boundary";
    var body = std.ArrayList(u8).init(allocator);
    defer body.deinit();

    try body.writer().print(
        "--{s}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"document.pdf\"\r\n" ++
            "Content-Type: application/pdf\r\n\r\n",
        .{boundary},
    );
    try body.appendSlice(file_bytes);
    try body.writer().print(
        "\r\n--{s}\r\nContent-Disposition: form-data; name=\"chunking\"\r\n\r\n" ++
            "{{\"max_characters\":800,\"overlap\":100}}\r\n--{s}--\r\n",
        .{ boundary, boundary },
    );

    var client = std.http.Client{ .allocator = allocator };
    defer client.deinit();

    const uri = try std.Uri.parse("http://localhost:8000/extract");
    var header_buf: [4096]u8 = undefined;
    var req = try client.open(.POST, uri, .{
        .server_header_buffer = &header_buf,
        .extra_headers = &.{
            .{ .name = "content-type", .value = "multipart/form-data; boundary=" ++ boundary },
        },
    });
    defer req.deinit();

    req.transfer_encoding = .{ .content_length = body.items.len };
    try req.send();
    try req.writeAll(body.items);
    try req.finish();
    try req.wait();

    const response_body = try req.reader().readAllAlloc(allocator, 16 * 1024 * 1024);
    defer allocator.free(response_body);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{response_body});
}
```
