```csharp title="C#"
using Xberg;

var config = new ExtractionConfig();
var result = XbergLib.ExtractSync("document.pdf", null, config);

if (result.Metadata?.Pages?.Boundaries != null)
{
    foreach (var boundary in result.Metadata.Pages.Boundaries.Take(3))
    {
        var pageStart = (int)boundary.ByteStart;
        var pageEnd = (int)boundary.ByteEnd;

        if (pageEnd > result.Content.Length)
            pageEnd = result.Content.Length;

        var pageText = result.Content.Substring(pageStart, pageEnd - pageStart);
        var previewEnd = Math.Min(100, pageText.Length);
        var preview = pageText.Substring(0, previewEnd);

        Console.WriteLine($"Page {boundary.PageNumber}:");
        Console.WriteLine($"  Byte range: {boundary.ByteStart}-{boundary.ByteEnd}");
        Console.WriteLine($"  Preview: {preview}...");
    }
}
```
