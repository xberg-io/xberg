```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxCharacters = 1000,
        Overlap = 200,
        Embedding = new EmbeddingConfig
        {
            Normalize = true,
            BatchSize = 16,
            ShowDownloadProgress = true,
            CacheDir = null
        }
    }
};

var result = await XbergLib.Extract("document.pdf", null, config);
if (result.Chunks != null)
{
    Console.WriteLine($"Chunks with embeddings: {result.Chunks.Count}");
}
```
