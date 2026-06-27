```csharp title="C#"
using Xberg;
using System.Collections.Concurrent;

var processor = new StatefulPostProcessor();
PostProcessorRegistry.Register(processor);

public class StatefulPostProcessor : IPostProcessor
{
    private int _callCount = 0;
    private readonly ConcurrentDictionary<string, string> _cache = new();

    public string Name => "stateful-processor";
    public string Version => "1.0.0";

    public void Initialize()
    {
        Console.WriteLine("Stateful processor initialized");
        _callCount = 0;
        _cache.Clear();
    }

    public void Shutdown()
    {
        Console.WriteLine($"Stateful processor called {_callCount} times");
        Console.WriteLine($"Cache contains {_cache.Count} entries");
    }

    public void Process(ExtractedDocument result, ExtractionConfig config)
    {
        _callCount++;

        var key = $"last_mime_{_callCount}";
        _cache.TryAdd(key, result.MimeType);

        Console.WriteLine($"Processing #{_callCount}: {result.MimeType}");
    }

    public ProcessingStage ProcessingStage()
    {
        return ProcessingStage.Middle;
    }

    public bool ShouldProcess(ExtractedDocument result, ExtractionConfig config)
    {
        return true;
    }

    public ulong EstimatedDurationMs(ExtractedDocument result)
    {
        return 5;
    }

    public int Priority()
    {
        return 50;
    }
}
```
