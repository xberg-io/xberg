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

var result = XbergLib.ExtractSync("document.pdf", null, config);

string content = result.Content;
string preview = content.Length > 100 ? content[..100] : content;
Console.WriteLine($"Extracted: {preview}");
```
