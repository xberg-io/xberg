```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Images = new ImageExtractionConfig
    {
        ExtractImages = true,
        TargetDpi = 200,
        MaxImageDimension = 2048,
        InjectPlaceholders = true, // set to false to extract images without markdown references
        AutoAdjustDpi = true
    }
};

var result = await XbergLib.ExtractAsync("document.pdf", config);
Console.WriteLine($"Extracted: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
