using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "auto",
        Language = "en"
    }
};

var result = KreuzbergLib.ExtractFileSync("document.pdf", config);
Console.WriteLine(result.Content);
