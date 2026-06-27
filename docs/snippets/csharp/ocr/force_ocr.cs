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

var result = XbergLib.Extract("document.pdf", config);
Console.WriteLine(result.Content);
