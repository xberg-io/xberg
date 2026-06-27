```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    TokenReduction = new TokenReductionOptions
    {
        Mode = "moderate",
        PreserveImportantWords = true,
    },
};

var result = await XbergLib.Extract("document.pdf", null, config);
Console.WriteLine($"Content length: {result.Content.Length}");
```
