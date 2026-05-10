```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxCharacters = 1024,
        Overlap = 100,
        Embedding = new EmbeddingConfig
        {
            Model = new EmbeddingModelType.Preset("balanced"),
            Normalize = true,
            BatchSize = 32,
            ShowDownloadProgress = false,
        },
    },
};
```
