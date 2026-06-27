```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "enabled" => true,
    "max_characters" => 512
  },
  "embeddings" => %{
    "enabled" => true
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)

# Process chunks with embeddings for semantic search
embedded_chunks = result.chunks
  |> Enum.with_index(1)
  |> Enum.map(fn {chunk, idx} ->
    %{
      "chunk_id" => idx,
      "content" => chunk["content"],
      "embedding" => chunk["embedding"],
      "page" => chunk["page"],
      "metadata" => %{
        "document" => "document.pdf",
        "chunk_index" => idx
      }
    }
  end)

IO.puts("Prepared #{length(embedded_chunks)} chunks with embeddings")
IO.inspect(embedded_chunks, label: "Embedded Chunks")
```
