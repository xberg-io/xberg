using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng"
    }
};

var result = XbergLib.Extract("document.pdf", config);
