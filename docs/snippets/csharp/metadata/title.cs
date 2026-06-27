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
    var title = result.Metadata.Format.Pdf.Title;
    Console.WriteLine($"Title: {title}");
}
