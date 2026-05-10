using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng+fra"
    }
};

var result = KreuzbergLib.ExtractFileSync("document.pdf", config);
