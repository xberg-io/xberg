```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    TokenReduction = new TokenReductionOptions
    {
        Mode = "moderate",
        PreserveImportantWords = true
    }
};

var result = await XbergLib.Extract("document.pdf", null, config);
Console.WriteLine($"Reduced content length: {result.Content.Length}");
Console.WriteLine($"Content: {result.Content.Substring(0, Math.Min(100, result.Content.Length))}");
```
