```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.9m,
        DetectMultiple = true
    }
};

var result = await KreuzbergLib.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"Languages: {string.Join(", ", result.DetectedLanguages ?? new List<string>())}");
```
