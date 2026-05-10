```csharp title="C#"
using Kreuzberg;
using System.Collections.Generic;

public class CustomEmbeddingBackend : IEmbeddingBackend
{
    public string Name => "custom-embeddings";
    public string Version => "1.0.0";

    public void Initialize()
    {
        Console.WriteLine("Embedding backend initialized");
    }

    public void Shutdown()
    {
        Console.WriteLine("Embedding backend shut down");
    }

    public ulong Dimensions()
    {
        return 384;
    }

    public List<List<float>> Embed(List<string> texts)
    {
        var embeddings = new List<List<float>>();
        foreach (var text in texts)
        {
            var embedding = new List<float>();
            for (int i = 0; i < 384; i++)
            {
                embedding.Add((float)(text.Length % (i + 1)) / (float)(i + 1));
            }
            embeddings.Add(embedding);
        }
        return embeddings;
    }
}

var backend = new CustomEmbeddingBackend();
EmbeddingBackendRegistry.Register(backend);
```
