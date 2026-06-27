```csharp title="C#"
using Xberg;

var processor = new WordCountProcessor();
PostProcessorRegistry.Register(processor);

public class WordCountProcessor : IPostProcessor
{
    public string Name => "word-count";
    public string Version => "1.0.0";

    public void Initialize()
    {
        Console.WriteLine("Word count processor initialized");
    }

    public void Shutdown()
    {
        Console.WriteLine("Word count processor shut down");
    }

    public void Process(ExtractedDocument result, ExtractionConfig config)
    {
        var wordCount = CountWords(result.Content);

        if (result.Metadata == null)
        {
            result.Metadata = new Metadata();
        }

        Console.WriteLine($"Document contains {wordCount} words");
    }

    public ProcessingStage ProcessingStage()
    {
        return ProcessingStage.Early;
    }

    public bool ShouldProcess(ExtractedDocument result, ExtractionConfig config)
    {
        return !string.IsNullOrEmpty(result.Content);
    }

    public ulong EstimatedDurationMs(ExtractedDocument result)
    {
        return 5;
    }

    public int Priority()
    {
        return 50;
    }

    private int CountWords(string content)
    {
        if (string.IsNullOrWhiteSpace(content))
            return 0;

        return content.Split(new[] { ' ', '\t', '\n', '\r' }, System.StringSplitOptions.RemoveEmptyEntries).Length;
    }
}
```
