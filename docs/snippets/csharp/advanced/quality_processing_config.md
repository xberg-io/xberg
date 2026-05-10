```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    EnableQualityProcessing = true
};

var result = await KreuzbergLib.ExtractFileAsync(
    "document.pdf",
    config
);

var qualityScore = result.QualityScore;

Console.WriteLine($"Quality score: {qualityScore:F2}");
```
