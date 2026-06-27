```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxCharacters = 1000,
        Overlap = 200,
        ChunkerType = ChunkerType.Text
    }
};

var result = await XbergLib.Extract("document.pdf", null, config);
if (result.Chunks != null)
{
    Console.WriteLine($"Total chunks: {result.Chunks.Count}");
    foreach (var chunk in result.Chunks)
    {
        Console.WriteLine($"Chunk length: {chunk.Content.Length}");
    }
}
```

```csharp title="C# - Markdown with Heading Context"
using Xberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxCharacters = 500,
        Overlap = 50,
        ChunkerType = ChunkerType.Markdown,
        PrependHeadingContext = true
    }
};

var result = await XbergLib.Extract("document.md", null, config);
if (result.Chunks != null)
{
    foreach (var chunk in result.Chunks)
    {
        Console.WriteLine($"Content: {chunk.Content.Substring(0, Math.Min(100, chunk.Content.Length))}");
    }
}
```
