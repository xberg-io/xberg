using Xberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "easyocr",
        Language = "en",
        UseGpu = true
    }
};

var result = XbergLib.ExtractSync("scanned.pdf", config);
Console.WriteLine(result.Content);
