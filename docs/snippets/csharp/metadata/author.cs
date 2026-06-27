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
    var author = result.Metadata.Format.Pdf.Author;
    Console.WriteLine($"Author: {author}");
}
