```csharp title="C#"
using Kreuzberg;

public class UnregisterableProcessor : IPostProcessor
{
    public string Name => "removable-processor";
    public string Version => "1.0.0";

    public void Initialize() { }
    public void Shutdown() { }

    public void Process(ExtractionResult result, ExtractionConfig config)
    {
        Console.WriteLine("Processing...");
    }

    public ProcessingStage ProcessingStage() => ProcessingStage.Middle;
    public bool ShouldProcess(ExtractionResult result, ExtractionConfig config) => true;
    public ulong EstimatedDurationMs(ExtractionResult result) => 10;
    public int Priority() => 50;
}

var processor = new UnregisterableProcessor();
PostProcessorRegistry.Register(processor);

Console.WriteLine("Processor registered");
var processors = KreuzbergLib.ListPostProcessors();
Console.WriteLine($"Active processors: {string.Join(", ", processors)}");

PostProcessorRegistry.Unregister(processor.Name);
Console.WriteLine("Processor unregistered");

processors = KreuzbergLib.ListPostProcessors();
Console.WriteLine($"Active processors: {string.Join(", ", processors)}");
```
