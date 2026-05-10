```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxCharacters = 1500,
        Overlap = 200,
        Embedding = new EmbeddingConfig
        {
            Model = new EmbeddingModelType.Preset("balanced"),
        },
    },
};
```
