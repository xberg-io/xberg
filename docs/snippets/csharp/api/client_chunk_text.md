```csharp title="C#"
using System.Net.Http;
using System.Net.Http.Json;
using System.Text.Json.Serialization;

public record ChunkRequest(
    [property: JsonPropertyName("text")] string Text,
    [property: JsonPropertyName("max_characters")] int? MaxCharacters = null,
    [property: JsonPropertyName("overlap")] int? Overlap = null,
    [property: JsonPropertyName("chunker_type")] string? ChunkerType = null
);

public record ChunkResponse(
    [property: JsonPropertyName("chunks")] List<ChunkItem> Chunks,
    [property: JsonPropertyName("chunk_count")] int ChunkCount
);

public record ChunkItem(
    [property: JsonPropertyName("content")] string Content,
    [property: JsonPropertyName("chunk_index")] int ChunkIndex
);

var client = new HttpClient();
var request = new ChunkRequest(
    Text: "Your long text content here...",
    MaxCharacters: 1000,
    Overlap: 50,
    ChunkerType: "text"
);

var response = await client.PostAsJsonAsync("http://localhost:8000/chunk", request);
var result = await response.Content.ReadFromJsonAsync<ChunkResponse>();

Console.WriteLine($"Created {result?.ChunkCount} chunks");
foreach (var chunk in result?.Chunks ?? [])
{
    Console.WriteLine($"Chunk {chunk.ChunkIndex}: {chunk.Content[..Math.Min(50, chunk.Content.Length)]}...");
}
```
