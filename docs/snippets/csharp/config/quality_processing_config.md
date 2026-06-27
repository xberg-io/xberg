```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    EnableQualityProcessing = true,
    UseCache = true
};

var result = await XbergLib.Extract("document.pdf", null, config);
Console.WriteLine($"Quality score: {result.QualityScore}");
Console.WriteLine($"Content length: {result.Content.Length}");
```
