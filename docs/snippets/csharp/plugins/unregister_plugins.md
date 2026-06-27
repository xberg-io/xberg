```csharp title="C#"
using Xberg;

var processor = new UnregisterableProcessor();
PostProcessorRegistry.Register(processor);

Console.WriteLine("Processor registered");
var processors = XbergLib.ListPostProcessors();
Console.WriteLine($"Active processors: {string.Join(", ", processors)}");

PostProcessorRegistry.Unregister(processor.Name);
Console.WriteLine("Processor unregistered");

processors = XbergLib.ListPostProcessors();
Console.WriteLine($"Active processors: {string.Join(", ", processors)}");

public class UnregisterableProcessor : IPostProcessor
{
    public string Name => "removable-processor";
    public string Version => "1.0.0";

    public void Initialize() { }
    public void Shutdown() { }

    public void Process(ExtractedDocument result, ExtractionConfig config)
    {
        Console.WriteLine("Processing...");
    }

    public ProcessingStage ProcessingStage() => ProcessingStage.Middle;
    public bool ShouldProcess(ExtractedDocument result, ExtractionConfig config) => true;
    public ulong EstimatedDurationMs(ExtractedDocument result) => 10;
    public int Priority() => 50;
}
```
