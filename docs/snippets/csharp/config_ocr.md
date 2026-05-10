```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng+fra",
        TesseractConfig = new TesseractConfig { Psm = 3 }
    }
};

var result = await KreuzbergLib.ExtractFileAsync("document.pdf", config);
Console.WriteLine(result.Content);
```
