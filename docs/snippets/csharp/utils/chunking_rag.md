```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxCharacters = 500,
        Overlap = 50,
        Embedding = new EmbeddingConfig
        {
            Model = new EmbeddingModelType.Preset("balanced"),
            Normalize = true,
            BatchSize = 16,
        },
    },
};

var result = await XbergLib.Extract("research_paper.pdf", null, config);

var chunksWithEmbeddings = new List<(string Preview, int Dimensions)>();
foreach (var chunk in result.Chunks ?? new List<Chunk>())
{
    if (chunk.Embedding is { Count: > 0 } embedding)
    {
        var preview = chunk.Content.Length > 100 ? chunk.Content[..100] : chunk.Content;
        chunksWithEmbeddings.Add((preview, embedding.Count));
    }
}

Console.WriteLine($"Chunks with embeddings: {chunksWithEmbeddings.Count}");
```
