```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.8,
        DetectMultiple = true
    }
};

var result = KreuzbergLib.ExtractFileSync("document.pdf", null, config);

if (result.DetectedLanguages != null && result.DetectedLanguages.Count > 0)
{
    Console.WriteLine($"Detected languages: {string.Join(", ", result.DetectedLanguages)}");
    foreach (var language in result.DetectedLanguages)
    {
        Console.WriteLine($"  - {language}");
    }
}
```
