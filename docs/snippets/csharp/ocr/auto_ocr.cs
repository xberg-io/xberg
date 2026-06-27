using Xberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "auto",
        Language = "en"
    }
};

var result = XbergLib.Extract("document.pdf", config);
Console.WriteLine(result.Content);
