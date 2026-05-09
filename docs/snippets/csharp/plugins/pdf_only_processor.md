```csharp title="C#"
using Kreuzberg;

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

    public void Process(ExtractionResult result, ExtractionConfig config)
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

    public bool ShouldProcess(ExtractionResult result, ExtractionConfig config)
    {
        return result.MimeType == "application/pdf";
    }

    public ulong EstimatedDurationMs(ExtractionResult result)
    {
        return 10;
    }

    public int Priority()
    {
        return 50;
    }
}

var processor = new PdfOnlyProcessor();
PostProcessorRegistry.Register(processor);
```
