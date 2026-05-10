```csharp title="C#"
using Kreuzberg;

var result = KreuzbergLib.ExtractFileSync("document.pdf", new ExtractionConfig());

foreach (var table in result.Tables)
{
    Console.WriteLine($"Table with {table.Rows.Count} rows");
}

foreach (var chunk in result.Chunks)
{
    Console.WriteLine(chunk.Content);
}
```
