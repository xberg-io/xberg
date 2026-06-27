```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    // Configuration is passed across the FFI as a JSON document.
    // This combines OCR, chunking, image extraction, output format, and caching.
    const config_json =
        \\{
        \\  "use_cache": true,
        \\  "enable_quality_processing": true,
        \\  "force_ocr": false,
        \\  "ocr": {
        \\    "backend": "tesseract",
        \\    "language": "eng"
        \\  },
        \\  "chunking": {
        \\    "max_characters": 800,
        \\    "overlap": 100,
        \\    "chunker_type": "markdown",
        \\    "prepend_heading_context": true
        \\  },
        \\  "images": {
        \\    "extract_images": true
        \\  },
        \\  "output_format": "markdown",
        \\  "include_document_structure": true
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"report.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("Result ({d} bytes of JSON):\n{s}\n", .{ output_json.len, output_json });
}
```
