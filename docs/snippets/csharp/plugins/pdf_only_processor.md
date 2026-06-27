```csharp title="C#"
using Xberg;

public class PdfOnlyProcessor : IPostProcessor
{
    public string Name => "pdf-only-processor";
    public string Version => "1.0.0";

    public void Initialize()
    {
    }

    public void Shutdown()
    {
    }

    public void Process(ExtractedDocument result, ExtractionConfig config)
    {
        if (result.MimeType != "application/pdf")
        {
            Console.WriteLine($"Skipping non-PDF: {result.MimeType}");
        }
    }

    public ProcessingStage ProcessingStage()
    {
        return ProcessingStage.Middle;
    }

    public bool ShouldProcess(ExtractedDocument result, ExtractionConfig config)
    {
        return result.MimeType == "application/pdf";
    }

    public ulong EstimatedDurationMs(ExtractedDocument result)
    {
        return 10;
    }

    public int Priority()
    {
        return 50;
    }
}

class Program
{
    static void Main()
    {
        var processor = new PdfOnlyProcessor();
        PostProcessorRegistry.Register(processor);
    }
}
```
