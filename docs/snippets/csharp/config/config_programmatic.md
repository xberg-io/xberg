```csharp title="C#"
using Kreuzberg;

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

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
Console.WriteLine($"Content length: {result.Content.Length}");
```
