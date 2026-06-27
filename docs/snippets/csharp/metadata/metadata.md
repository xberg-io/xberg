```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    PdfOptions = new PdfConfig { ExtractMetadata = true }
};

var result = XbergLib.ExtractSync("document.pdf", null, config);

if (result.Metadata?.Format?.Pdf != null)
{
    var pdfMeta = result.Metadata.Format.Pdf;
    Console.WriteLine($"Pages: {pdfMeta.PageCount}");
    Console.WriteLine($"Author: {pdfMeta.Author}");
    Console.WriteLine($"Title: {pdfMeta.Title}");
}

var htmlResult = XbergLib.ExtractSync("page.html", null, config);
if (htmlResult.Metadata?.Format?.Html != null)
{
    var htmlMeta = htmlResult.Metadata.Format.Html;
    Console.WriteLine($"Title: {htmlMeta.Title}");
    Console.WriteLine($"Description: {htmlMeta.Description}");

    if (htmlMeta.Keywords != null && htmlMeta.Keywords.Count > 0)
    {
        Console.WriteLine($"Keywords: {string.Join(", ", htmlMeta.Keywords)}");
    }

    if (htmlMeta.CanonicalUrl != null)
    {
        Console.WriteLine($"Canonical URL: {htmlMeta.CanonicalUrl}");
    }

    if (htmlMeta.OpenGraph != null && htmlMeta.OpenGraph.Count > 0)
    {
        if (htmlMeta.OpenGraph.ContainsKey("image"))
            Console.WriteLine($"Open Graph Image: {htmlMeta.OpenGraph["image"]}");
        if (htmlMeta.OpenGraph.ContainsKey("title"))
            Console.WriteLine($"Open Graph Title: {htmlMeta.OpenGraph["title"]}");
    }

    if (htmlMeta.TwitterCard != null && htmlMeta.TwitterCard.Count > 0)
    {
        if (htmlMeta.TwitterCard.ContainsKey("card"))
            Console.WriteLine($"Twitter Card Type: {htmlMeta.TwitterCard["card"]}");
    }

    if (htmlMeta.Language != null)
        Console.WriteLine($"Language: {htmlMeta.Language}");

    if (htmlMeta.Headers != null && htmlMeta.Headers.Count > 0)
        Console.WriteLine($"Headers: {string.Join(", ", htmlMeta.Headers.Select(h => h.Text))}");

    if (htmlMeta.Links != null && htmlMeta.Links.Count > 0)
    {
        foreach (var link in htmlMeta.Links)
            Console.WriteLine($"Link: {link.Href} ({link.Text})");
    }

    if (htmlMeta.Images != null && htmlMeta.Images.Count > 0)
        Console.WriteLine($"Images: {string.Join(", ", htmlMeta.Images.Select(i => i.Src))}");
}
```
