```csharp title="C#"
using Xberg;

var extractor = new JsonDocumentExtractor();
XbergLib.RegisterDocumentExtractor(extractor);

public class JsonDocumentExtractor : IDocumentExtractor
{
    public string Name => "json-extractor";
    public string Version => "1.0.0";

    public void Initialize()
    {
        Console.WriteLine("JSON extractor initialized");
    }

    public void Shutdown()
    {
        Console.WriteLine("JSON extractor shut down");
    }

    public ExtractionResult Extract(byte[] content, string mimeType, ExtractionConfig config)
    {
        var json = System.Text.Encoding.UTF8.GetString(content);

        var result = new ExtractionResult
        {
            Content = json,
            MimeType = mimeType,
            DetectedLanguages = null
        };
        return result;
    }

    public ExtractionResult Extract(string path, string mimeType, ExtractionConfig config)
    {
        var content = System.IO.File.ReadAllBytes(path);
        return Extract(content, mimeType, config);
    }

    public string[] SupportedMimeTypes()
    {
        return new[] { "application/json", "text/json" };
    }

    public int Priority()
    {
        return 50;
    }
}
```
