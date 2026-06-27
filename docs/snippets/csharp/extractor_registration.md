```csharp title="C#"
using Xberg;

var extractor = new CustomExtractor();
XbergLib.RegisterDocumentExtractor(extractor);
Console.WriteLine("Extractor registered");

public class CustomExtractor : IDocumentExtractor
{
    public string Name() => "custom";
    public string Version() => "1.0.0";

    public Dictionary<string, object> Extract(byte[] data, string mimeType, Dictionary<string, object> config)
    {
        return new Dictionary<string, object>
        {
            { "content", "Extracted content" },
            { "mime_type", mimeType }
        };
    }
}
```
