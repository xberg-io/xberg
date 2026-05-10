```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Postprocessor = new PostProcessorConfig
    {
        Enabled = true,
        EnabledProcessors = new List<string> { "deduplication" }
    }
};

var result = await KreuzbergLib.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"Content: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
