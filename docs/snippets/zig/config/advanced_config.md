```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config_json =
        \\{
        \\  "use_cache": true,
        \\  "enable_quality_processing": true,
        \\  "ocr": {
        \\    "backend": "tesseract",
        \\    "language": "eng"
        \\  },
        \\  "chunking": {
        \\    "max_characters": 1000,
        \\    "overlap": 200,
        \\    "embedding": {
        \\      "model": {"type": "preset", "name": "balanced"},
        \\      "batch_size": 32,
        \\      "normalize": true
        \\    }
        \\  },
        \\  "language_detection": {
        \\    "enabled": true,
        \\    "min_confidence": 0.8,
        \\    "detect_multiple": false
        \\  },
        \\  "keywords": {
        \\    "algorithm": "yake",
        \\    "max_keywords": 10,
        \\    "min_score": 0.1,
        \\    "ngram_range": [1, 3],
        \\    "language": "en"
        \\  },
        \\  "token_reduction": {
        \\    "mode": "moderate",
        \\    "preserve_important_words": true
        \\  },
        \\  "postprocessor": {
        \\    "enabled": true
        \\  }
        \\}
    ;

    const result_json = try xberg.extract_sync("document.pdf", null, config_json);
    defer std.heap.c_allocator.free(result_json);

    const owned = try allocator.dupe(u8, result_json);
    defer allocator.free(owned);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{owned});
}
```
