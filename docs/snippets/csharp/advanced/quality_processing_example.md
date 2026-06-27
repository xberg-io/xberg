```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    EnableQualityProcessing = true
};

var result = XbergLib.Extract(
    "scanned_document.pdf",
    config
);

var qualityScore = result.QualityScore;

if (qualityScore < 0.5)
{
    Console.WriteLine(
        $"Warning: Low quality extraction ({qualityScore:F2})"
    );
    Console.WriteLine(
        "Consider re-scanning with higher DPI or adjusting OCR settings"
    );
}
else
{
    Console.WriteLine($"Quality score: {qualityScore:F2}");
}
```
