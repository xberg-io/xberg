```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "enabled" => true,
    "max_characters" => 512,
    "overlap" => 50,
    "respect_boundaries" => true
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)

# Prepare chunks for vector embedding and search
chunks_for_embedding = result.chunks
  |> Enum.map(fn chunk ->
    %{
      "id" => chunk["id"],
      "content" => chunk["content"],
      "metadata" => %{
        "page" => chunk["page"],
        "source" => "document.pdf"
      }
    }
  end)

IO.inspect(chunks_for_embedding, label: "Chunks Ready for RAG")
```
