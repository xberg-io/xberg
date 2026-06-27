```csharp title="detect_language.cs"
using Xberg;
using System;
using System.Collections.Generic;

var config = new ExtractionConfig
{
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.9,
        DetectMultiple = false
    }
};

var result = XbergLib.ExtractSync("document.pdf", config);

Console.WriteLine("Detected Language:");
foreach (var lang in result.DetectedLanguages)
{
    Console.WriteLine($"  - {lang}");
}

var multiLangConfig = new ExtractionConfig
{
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.8,
        DetectMultiple = true
    }
};

var multiResult = XbergLib.ExtractSync("multilingual_document.pdf", multiLangConfig);

Console.WriteLine("Detected Languages:");
foreach (var lang in multiResult.DetectedLanguages)
{
    Console.WriteLine($"  - {lang}");
}

Console.WriteLine($"\nLanguage Detection Summary:");
Console.WriteLine($"  - Content: {multiResult.Content.Substring(0, 100)}...");
Console.WriteLine($"  - Languages: {string.Join(", ", multiResult.DetectedLanguages)}");
Console.WriteLine($"  - Quality Score: {multiResult.Metadata.QualityScore}");
```
