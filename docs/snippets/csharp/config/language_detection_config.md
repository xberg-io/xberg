```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.8,
        DetectMultiple = true
    }
};

var result = await XbergLib.Extract("document.pdf", null, config);
Console.WriteLine($"Detected language: {result.Language}");
if (result.DetectedLanguages != null)
{
    Console.WriteLine($"All detected: {string.Join(", ", result.DetectedLanguages)}");
}
```
