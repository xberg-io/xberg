```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Images = new ImageExtractionConfig
    {
        ExtractImages = true,
        TargetDpi = 300,
        MaxImageDimension = 4096,
        AutoAdjustDpi = true,
        MinDpi = 150,
        MaxDpi = 600
    }
};

var result = await XbergLib.Extract("document.pdf", null, config);
if (result.Images != null)
{
    Console.WriteLine($"Extracted images: {result.Images.Count}");
}
```
