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

var result = XbergLib.ExtractSync("scanned.pdf", null, config);

string content = result.Content;
string preview = content.Length > 100 ? content[..100] : content;
int totalLength = content.Length;

Console.WriteLine($"Extracted content (preview): {preview}");
Console.WriteLine($"Total characters: {totalLength}");
```
