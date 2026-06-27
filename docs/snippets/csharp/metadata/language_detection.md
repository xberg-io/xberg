```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.9,
        DetectMultiple = false
    }
};

var result = XbergLib.ExtractSync("document.pdf", null, config);

if (result.DetectedLanguages != null && result.DetectedLanguages.Count > 0)
{
    Console.WriteLine($"Primary language: {result.DetectedLanguages[0]}");
}
```
