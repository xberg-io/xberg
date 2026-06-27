```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        Enabled = true,
        ChunkSize = 512,
        OverlapSize = 50
    },
    Embeddings = new EmbeddingConfig
    {
        Enabled = true
    }
};

var result = XbergLib.ExtractSync("document.pdf", null, config);

if (result.Chunks != null)
{
    foreach (var chunk in result.Chunks)
    {
        Console.WriteLine($"Chunk: {chunk.Text.Substring(0, Math.Min(50, chunk.Text.Length))}...");

        if (chunk.Embeddings != null && chunk.Embeddings.Count > 0)
        {
            Console.WriteLine($"  Embedding dimensions: {chunk.Embeddings.Count}");
            Console.WriteLine($"  First values: {string.Join(", ", chunk.Embeddings.Take(5))}");
        }

        if (chunk.Metadata != null)
        {
            if (chunk.Metadata.ContainsKey("page_number"))
                Console.WriteLine($"  Page: {chunk.Metadata["page_number"]}");
            if (chunk.Metadata.ContainsKey("token_count"))
                Console.WriteLine($"  Tokens: {chunk.Metadata["token_count"]}");
        }
    }
}
```
