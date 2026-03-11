```csharp title="C#"
using Kreuzberg;

class Program
{
    static async Task Main()
    {
        var config = new ExtractionConfig
        {
            Chunking = new ChunkingConfig
            {
                MaxChars = 1000,
                MaxOverlap = 200,
                Embedding = new EmbeddingConfig
                {
                    Model = EmbeddingModelType.Preset("all-minilm-l6-v2"),
                    Normalize = true,
                    BatchSize = 32
                }
            }
        };

        try
        {
            var result = await KreuzbergClient.ExtractFileAsync(
                "document.pdf",
                config
            ).ConfigureAwait(false);

            Console.WriteLine($"Chunks: {result.Chunks.Count}");
            foreach (var chunk in result.Chunks)
            {
                Console.WriteLine($"Content length: {chunk.Content.Length}");
                if (chunk.Embedding != null)
                {
                    Console.WriteLine($"Embedding dimensions: {chunk.Embedding.Length}");
                }
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
```

```csharp title="C# - Markdown with Heading Context"
using Kreuzberg;

class Program
{
    static async Task Main()
    {
        var config = new ExtractionConfig
        {
            Chunking = new ChunkingConfig
            {
                MaxChars = 500,
                MaxOverlap = 50,
                Sizing = new ChunkSizingConfig
                {
                    Type = "tokenizer",
                    Model = "Xenova/gpt-4o"
                }
            }
        };

        try
        {
            var result = await KreuzbergClient.ExtractFileAsync(
                "document.md",
                config
            ).ConfigureAwait(false);

            foreach (var chunk in result.Chunks)
            {
                if (chunk.HeadingContext?.Headings != null)
                {
                    Console.WriteLine("Headings:");
                    foreach (var heading in chunk.HeadingContext.Headings)
                    {
                        Console.WriteLine($"  Level {heading.Level}: {heading.Text}");
                    }
                }
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
```
