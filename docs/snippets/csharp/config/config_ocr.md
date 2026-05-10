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

var result = await KreuzbergLib.ExtractFile("scanned.pdf", null, config);
Console.WriteLine($"Content length: {result.Content.Length}");
if (result.Tables != null)
{
    Console.WriteLine($"Tables detected: {result.Tables.Count}");
}
```
