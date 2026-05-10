```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng"
    }
};

var result = KreuzbergLib.ExtractFileSync("scanned.pdf", config);
Console.WriteLine(result.Content);
```
