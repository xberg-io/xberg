```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "paddle-ocr",
        Language = "en",
        // PaddleOcrConfig = new PaddleOcrConfig { ModelTier = "server" } // for max accuracy
    }
};

var result = KreuzbergLib.ExtractFileSync("scanned.pdf", config);
Console.WriteLine(result.Content);
```
