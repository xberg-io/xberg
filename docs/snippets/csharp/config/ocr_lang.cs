using Xberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng+fra"
    }
};

var result = XbergLib.ExtractSync("document.pdf", config);
