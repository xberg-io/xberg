```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Postprocessor = new PostProcessorConfig
    {
        Enabled = true,
        EnabledProcessors = new List<string>
        {
            "whitespace_normalizer",
            "unicode_normalizer"
        },
        DisabledProcessors = null
    }
};

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
Console.WriteLine($"Processed content: {result.Content.Substring(0, Math.Min(100, result.Content.Length))}");
```
