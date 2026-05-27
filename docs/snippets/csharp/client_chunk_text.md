```csharp title="C#"
using System.Net.Http.Json;
using System.Text.Json;
using System.Text.Json.Serialization;

// Request models
public record ChunkRequest(
    [property: JsonPropertyName("text")] string Text,
    [property: JsonPropertyName("chunker_type")] string? ChunkerType = null,
    [property: JsonPropertyName("config")] ChunkConfig? Config = null
);

public record ChunkConfig(
    [property: JsonPropertyName("max_characters")] int? MaxCharacters = null,
    [property: JsonPropertyName("overlap")] int? Overlap = null,
    [property: JsonPropertyName("trim")] bool? Trim = null
);

// Response models
public record ChunkResponse(
    [property: JsonPropertyName("chunks")] List<ChunkItem> Chunks,
    [property: JsonPropertyName("chunk_count")] int ChunkCount,
    [property: JsonPropertyName("input_size_bytes")] int InputSizeBytes,
    [property: JsonPropertyName("chunker_type")] string ChunkerType
);

public record ChunkItem(
    [property: JsonPropertyName("content")] string Content,
    [property: JsonPropertyName("byte_start")] int ByteStart,
    [property: JsonPropertyName("byte_end")] int ByteEnd,
    [property: JsonPropertyName("chunk_index")] int ChunkIndex,
    [property: JsonPropertyName("total_chunks")] int TotalChunks,
    [property: JsonPropertyName("first_page")] int? FirstPage,
    [property: JsonPropertyName("last_page")] int? LastPage
);

class Program
{
    static async Task Main()
    {
        using var client = new HttpClient();

        var request = new ChunkRequest(
            Text: "Your long text content here...",
            ChunkerType: "text",
            Config: new ChunkConfig(
                MaxCharacters: 1000,
                Overlap: 50,
                Trim: true
            )
        );

        var response = await client.PostAsJsonAsync(
            "http://localhost:8000/chunk",
            request
        );

        var result = await response.Content.ReadFromJsonAsync<ChunkResponse>();

        Console.WriteLine($"Created {result?.ChunkCount} chunks");
        foreach (var chunk in result?.Chunks ?? [])
        {
            var preview = chunk.Content[..Math.Min(50, chunk.Content.Length)];
            Console.WriteLine($"Chunk {chunk.ChunkIndex}: {preview}...");
        }
    }
}
```
