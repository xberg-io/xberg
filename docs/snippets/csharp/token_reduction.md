```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    TokenReduction = new TokenReductionConfig
    {
        Mode = "moderate",
        PreserveImportantWords = true
    }
};

var result = await XbergLib.ExtractAsync("document.pdf", config);
Console.WriteLine($"Content length: {result.Content.Length}");
```
