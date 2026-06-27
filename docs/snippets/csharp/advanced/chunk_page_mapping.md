```csharp title="C#"
using Xberg;

class Program
{
    static async Task Main()
    {
        var config = new ExtractionConfig
        {
            Chunking = new ChunkingConfig
            {
                MaxCharacters = 500,
                Overlap = 50
            },
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        try
        {
            var result = await XbergLib.ExtractAsync(
                "document.pdf",
                config
            ).ConfigureAwait(false);

            if (result.Chunks != null)
            {
                foreach (var chunk in result.Chunks)
                {
                    if (chunk.Metadata.FirstPage.HasValue && chunk.Metadata.LastPage.HasValue)
                    {
                        var first = chunk.Metadata.FirstPage.Value;
                        var last = chunk.Metadata.LastPage.Value;
                        var pageRange = first == last
                            ? $"Page {first}"
                            : $"Pages {first}-{last}";

                        var preview = chunk.Content[..Math.Min(50, chunk.Content.Length)];
                        Console.WriteLine($"Chunk: {preview}... ({pageRange})");
                    }
                }
            }
        }
        catch (XbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
```
