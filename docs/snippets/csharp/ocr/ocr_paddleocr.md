```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "paddle-ocr",
        Language = "en",
        // PaddleOcrConfig = new PaddleOcrConfig { ModelTier = "server" } // for max accuracy
    }
};

var result = XbergLib.ExtractSync("scanned.pdf", config);
Console.WriteLine(result.Content);
```
