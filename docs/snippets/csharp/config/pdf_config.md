```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    PdfOptions = new PdfConfig
    {
        ExtractImages = true,
        ExtractMetadata = true,
        ExtractAnnotations = false,
        Passwords = new List<string> { "password123" }
    }
};

var result = await XbergLib.Extract("encrypted.pdf", null, config);
if (result.Metadata != null)
{
    Console.WriteLine($"Title: {result.Metadata.Title}");
    Console.WriteLine($"Authors: {string.Join(", ", result.Metadata.Authors ?? new List<string>())}");
}
```
