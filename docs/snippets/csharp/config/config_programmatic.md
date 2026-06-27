```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true,
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng+deu",
        TesseractConfig = new TesseractConfig
        {
            Psm = 6
        }
    },
    Chunking = new ChunkingConfig
    {
        MaxCharacters = 1000,
        Overlap = 200
    }
};

var result = await XbergLib.Extract("document.pdf", null, config);
Console.WriteLine($"Content length: {result.Content.Length}");
```
