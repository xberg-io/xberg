```csharp title="C#"
using Xberg;

public class PdfOnlyProcessor : IPostProcessor
{
    public string Name() => "pdf-only-processor";
    public string Version() => "1.0.0";

    public ExtractedDocument Process(ExtractedDocument result) => result;

    public bool ShouldProcess(ExtractedDocument result)
        => result.MimeType == "application/pdf";
}

class Program
{
    static void Main()
    {
        var processor = new PdfOnlyProcessor();
        XbergLib.RegisterPostProcessor(processor);
    }
}
```
