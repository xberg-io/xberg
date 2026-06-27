```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        TesseractConfig = new TesseractConfig
        {
            Preprocessing = new ImagePreprocessingConfig
            {
                TargetDpi = 300,
                Denoise = true,
                Deskew = true,
                ContrastEnhance = true,
                BinarizationMethod = "otsu"
            }
        }
    }
};

var result = await XbergLib.ExtractAsync("scanned.pdf", config);
Console.WriteLine($"Content: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
