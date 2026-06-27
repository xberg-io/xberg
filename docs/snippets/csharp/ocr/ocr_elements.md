```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "paddle-ocr",
        Language = "en"
    }
};

var result = XbergLib.ExtractSync("scanned.pdf", config);

if (result.OcrElements is not null)
{
    foreach (var element in result.OcrElements)
    {
        Console.WriteLine($"Text: {element.Text}");
        Console.WriteLine($"Confidence: {element.Confidence.Recognition:F2}");
        Console.WriteLine($"Geometry: {element.Geometry}");
        if (element.Rotation is not null)
        {
            Console.WriteLine($"Rotation: {element.Rotation.Angle}°");
        }
        Console.WriteLine();
    }
}
```
