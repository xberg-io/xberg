```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxCharacters = 512,
        Overlap = 50,
        Embedding = new EmbeddingConfig
        {
            Model = new EmbeddingModelType.Preset("balanced"),
            Normalize = true,
        },
    },
};

var result = await XbergLib.Extract("document.pdf", null, config);

var chunks = result.Chunks ?? new List<Chunk>();
for (var i = 0; i < chunks.Count; i++)
{
    var chunkId = $"doc_chunk_{i}";
    var preview = chunks[i].Content.Length > 50
        ? chunks[i].Content[..50]
        : chunks[i].Content;
    Console.WriteLine($"Chunk {chunkId}: {preview}");
}
```
