<!-- snippet:syntax-only -->

```zig title="Zig"
const std = @import("std");
const xberg = @import("xberg");

// Structured extraction is configured via the JSON `structured_extraction`
// field on `ExtractionConfig`. The schema is a JSON Schema string and
// `llm.model` selects the provider via liter-llm.
pub fn main() !void {
    const config_json =
        \\{
        \\  "structured_extraction": {
        \\    "schema": "{\"type\":\"object\",\"properties\":{\"title\":{\"type\":\"string\"},\"authors\":{\"type\":\"array\",\"items\":{\"type\":\"string\"}},\"date\":{\"type\":\"string\"}},\"required\":[\"title\",\"authors\",\"date\"],\"additionalProperties\":false}",
        \\    "schema_name": "Paper",
        \\    "strict": true,
        \\    "llm": {
        \\      "model": "openai/gpt-4o-mini"
        \\    }
        \\  }
        \\}
    ;

    const input_json = "{\"kind\":\"uri\",\"uri\":\"paper.pdf\"}";
    const output_json = try xberg.extract(input_json, config_json);
    defer std.heap.c_allocator.free(output_json);

    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{output_json});
}
```
