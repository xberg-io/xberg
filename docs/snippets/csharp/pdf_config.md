```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    PdfOptions = new PdfConfig
    {
        ExtractImages = true,
        ExtractMetadata = true,
        Passwords = new List<string> { "password1", "password2" },
        Hierarchy = new HierarchyConfig
        {
            Enabled = true,
            KClusters = 6,
            IncludeBbox = true,
            OcrCoverageThreshold = 0.5f
        }
    }
};

var result = await KreuzbergLib.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"Content: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
