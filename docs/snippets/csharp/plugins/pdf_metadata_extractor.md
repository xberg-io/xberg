```csharp title="C#"
using Xberg;

var enricher = new PdfMetadataEnricher();
PostProcessorRegistry.Register(enricher);

public class PdfMetadataEnricher : IPostProcessor
{
    private int _processedCount = 0;

    public string Name => "pdf-metadata-enricher";
    public string Version => "1.0.0";

    public void Initialize()
    {
        Console.WriteLine("PDF metadata enricher initialized");
        _processedCount = 0;
    }

    public void Shutdown()
    {
        Console.WriteLine($"PDF metadata enricher processed {_processedCount} documents");
    }

    public void Process(ExtractedDocument result, ExtractionConfig config)
    {
        if (result.MimeType == "application/pdf")
        {
            _processedCount++;
            if (result.Metadata == null)
            {
                result.Metadata = new Metadata();
            }
            result.Metadata.Author = result.Metadata.Author ?? "Unknown";
        }
    }

    public ProcessingStage ProcessingStage()
    {
        return ProcessingStage.Early;
    }

    public bool ShouldProcess(ExtractedDocument result, ExtractionConfig config)
    {
        return result.MimeType == "application/pdf";
    }

    public ulong EstimatedDurationMs(ExtractedDocument result)
    {
        return 50;
    }

    public int Priority()
    {
        return 50;
    }
}
```
