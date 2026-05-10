```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    TokenReduction = new TokenReductionOptions
    {
        Mode = "moderate",
        PreserveImportantWords = true
    }
};

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
Console.WriteLine($"Reduced content length: {result.Content.Length}");
Console.WriteLine($"Content: {result.Content.Substring(0, Math.Min(100, result.Content.Length))}");
```
