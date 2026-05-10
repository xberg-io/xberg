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

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
Console.WriteLine($"Detected language: {result.Language}");
if (result.DetectedLanguages != null)
{
    Console.WriteLine($"All detected: {string.Join(", ", result.DetectedLanguages)}");
}
```
