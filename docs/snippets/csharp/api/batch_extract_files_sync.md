```csharp title="C#"
using Kreuzberg;

var items = new List<BatchFileItem>
{
    new() { Path = "document1.pdf", Config = null },
    new()
    {
        Path = "document2.pdf",
        Config = new FileExtractionConfig { ForceOcr = true }
    }
};

var config = new ExtractionConfig { OutputFormat = OutputFormat.Text };
var results = KreuzbergLib.BatchExtractFilesSync(items, config);

foreach (var result in results)
{
    Console.WriteLine($"Content length: {result.Content.Length}");
}
```
