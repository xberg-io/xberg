```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "easyocr",
        Language = "en"
    }
};

// EasyOCR-specific options (use_gpu, beam_width, etc.) can be passed through
// OcrConfig's EasyocrConfig field if available, or via backend-specific configuration.
var result = XbergLib.ExtractSync("scanned.pdf", null, config);

string content = result.Content;
string preview = content.Length > 100 ? content[..100] : content;
int totalLength = content.Length;

Console.WriteLine($"Extracted content (preview): {preview}");
Console.WriteLine($"Total characters: {totalLength}");
```
