using Xberg;

var config = new ExtractionConfig
{
    ForceOcr = true,
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng"
    }
};

var result = XbergLib.ExtractSync("document.pdf", config);
Console.WriteLine(result.Content);
