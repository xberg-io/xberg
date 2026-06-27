```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng+deu",
        TesseractConfig = new TesseractConfig
        {
            Psm = 6,
            Oem = 3,
            MinConfidence = 0.5,
            Language = "eng"
        }
    }
};

var result = await XbergLib.Extract("scanned.pdf", null, config);
Console.WriteLine($"OCR text: {result.Content.Substring(0, Math.Min(100, result.Content.Length))}");
```
