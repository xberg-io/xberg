```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Postprocessor = new PostProcessorConfig
    {
        Enabled = true,
        EnabledProcessors = new List<string> { "deduplication" }
    }
};

var result = await XbergLib.ExtractAsync("document.pdf", config);
Console.WriteLine($"Content: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
