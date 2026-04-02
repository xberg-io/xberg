```csharp title="C#"
using Kreuzberg;

var client = new KreuzbergClient();

var config = new EmbeddingConfig { Model = EmbeddingModelType.Preset("balanced"), Normalize = true };
var texts = new[] { "Hello, world!", "Kreuzberg is fast" };

// Synchronous
var embeddings = client.EmbedSync(texts, config).ToList();
Console.WriteLine(embeddings.Count);       // 2
Console.WriteLine(embeddings[0].Length);   // 768

// Asynchronous
var asyncEmbeddings = await client.EmbedAsync(texts, config);
Console.WriteLine(asyncEmbeddings.First().Length); // 768
```
