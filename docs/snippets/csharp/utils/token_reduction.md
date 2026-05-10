```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    TokenReduction = new TokenReductionOptions
    {
        Mode = "moderate",
        PreserveImportantWords = true,
    },
};

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
Console.WriteLine($"Content length: {result.Content.Length}");
```
