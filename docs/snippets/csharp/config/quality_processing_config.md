```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    EnableQualityProcessing = true,
    UseCache = true
};

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
Console.WriteLine($"Quality score: {result.QualityScore}");
Console.WriteLine($"Content length: {result.Content.Length}");
```
