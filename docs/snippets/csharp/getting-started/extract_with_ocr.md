```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng"
    }
};

var result = XbergLib.Extract("scanned.pdf", config);
Console.WriteLine(result.Content);
```
