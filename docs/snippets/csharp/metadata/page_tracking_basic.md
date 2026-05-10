```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Pages = new PageConfig
    {
        ExtractPages = true
    }
};

var result = KreuzbergLib.ExtractFileSync("document.pdf", null, config);

if (result.Pages != null)
{
    foreach (var page in result.Pages)
    {
        Console.WriteLine($"Page {page.PageNumber}:");
        Console.WriteLine($"  Content: {page.Content.Length} chars");
        Console.WriteLine($"  Tables: {page.Tables.Count}");
        Console.WriteLine($"  Images: {page.Images.Count}");
    }
}
```
