```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    OutputFormat = OutputFormat.Markdown,
    UseCache = true,
    Ocr = new OcrConfig
    {
        Enabled = true,
        Backend = OcrBackendType.Tesseract,
        Languages = ["eng"]
    },
    ImageExtraction = new ImageExtractionConfig
    {
        Enabled = true,
        MinImageHeight = 100,
        MinImageWidth = 100
    },
    Chunking = new ChunkingConfig
    {
        Enabled = true,
        ChunkerType = ChunkerType.Text,
        MaxCharacters = 2000,
        Overlap = 100
    },
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true
    }
};

try
{
    var result = await XbergLib.Extract("document.pdf", null, config);
    Console.WriteLine($"Content: {result.Content}");
    Console.WriteLine($"Language: {result.Metadata?.LanguageDetection}");
    Console.WriteLine($"Format: {result.OutputFormat}");
}
catch (XbergException ex)
{
    Console.WriteLine($"Extraction error: {ex.Message}");
}
```
