```csharp title="C#"
using Kreuzberg;

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

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
if (result.Images != null)
{
    Console.WriteLine($"Extracted images: {result.Images.Count}");
}
```
