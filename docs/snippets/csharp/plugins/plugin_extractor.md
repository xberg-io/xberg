```csharp title="C#"
using Xberg;

var extractor = new CustomTextExtractor();
XbergLib.RegisterDocumentExtractor(extractor);

public class CustomTextExtractor : IDocumentExtractor
{
    public string Name => "custom-text-extractor";
    public string Version => "1.0.0";

    public void Initialize()
    {
        Console.WriteLine("Custom text extractor initialized");
    }

    public void Shutdown()
    {
        Console.WriteLine("Custom text extractor shut down");
    }

    public ExtractionResult Extract(byte[] content, string mimeType, ExtractionConfig config)
    {
        var text = System.Text.Encoding.UTF8.GetString(content);

        return new ExtractionResult
        {
            Content = text.ToUpper(),
            MimeType = mimeType,
            DetectedLanguages = null
        };
    }

    public ExtractionResult Extract(string path, string mimeType, ExtractionConfig config)
    {
        var content = System.IO.File.ReadAllBytes(path);
        return Extract(content, mimeType, config);
    }

    public string[] SupportedMimeTypes()
    {
        return new[] { "text/plain" };
    }

    public int Priority()
    {
        return 50;
    }
}
```
