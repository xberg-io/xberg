using Xberg;

var config = new ExtractionConfig
{
    PdfOptions = new PdfConfig
    {
        ExtractMetadata = true
    }
};

var result = XbergLib.ExtractSync("document.pdf", config);

if (result.Metadata?.Format.Pdf != null)
{
    var pdfMeta = result.Metadata.Format.Pdf;
    Console.WriteLine($"Pages: {pdfMeta.PageCount}");
    Console.WriteLine($"Author: {pdfMeta.Author}");
    Console.WriteLine($"Title: {pdfMeta.Title}");
    Console.WriteLine($"Subject: {pdfMeta.Subject}");
    Console.WriteLine($"Created: {pdfMeta.CreatedDate:O}");
}

var htmlResult = XbergLib.ExtractSync("page.html", config);
if (htmlResult.Metadata?.Format.Html != null)
{
    var htmlMeta = htmlResult.Metadata.Format.Html;
    Console.WriteLine($"Title: {htmlMeta.Title}");
    Console.WriteLine($"Description: {htmlMeta.Description}");
    if (htmlMeta.OpenGraph != null && htmlMeta.OpenGraph.ContainsKey("image"))
        Console.WriteLine($"Open Graph Image: {htmlMeta.OpenGraph["image"]}");
}
