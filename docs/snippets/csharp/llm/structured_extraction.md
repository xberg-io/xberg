<!-- snippet:syntax-only -->

```csharp title="C#"
using Xberg;
using System.Text.Json;
using System.Text.Json.Nodes;

var schema = JsonNode.Parse("""
{
    "type": "object",
    "properties": {
        "title": { "type": "string" },
        "authors": { "type": "array", "items": { "type": "string" } },
        "date": { "type": "string" }
    },
    "required": ["title", "authors", "date"],
    "additionalProperties": false
}
""")!;

var config = new ExtractionConfig
{
    StructuredExtraction = new StructuredExtractionConfig
    {
        Schema = schema,
        SchemaName = "paper_metadata",
        Strict = true,
        Llm = new LlmConfig
        {
            Model = "openai/gpt-4o-mini",
        },
    },
};

var result = await XbergLib.Extract("paper.pdf", null, config);

if (result.StructuredOutput is not null)
{
    Console.WriteLine(JsonSerializer.Serialize(result.StructuredOutput));
}
```
