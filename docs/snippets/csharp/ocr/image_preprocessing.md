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

var result = XbergLib.ExtractSync("scanned.pdf", null, config);

string content = result.Content;
string preview = content.Length > 100 ? content[..100] : content;
Console.WriteLine($"Content: {preview}");
```
