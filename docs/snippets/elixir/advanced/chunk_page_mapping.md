```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "enabled" => true,
    "max_characters" => 1024,
    "overlap" => 128
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)

# Map chunks to their source pages
chunks_with_pages = result.chunks
  |> Enum.map(fn chunk ->
    %{
      "chunk_id" => chunk["id"],
      "content" => chunk["content"],
      "page_number" => chunk["page"]
    }
  end)

IO.inspect(chunks_with_pages, label: "Chunks with Page Mapping")
```
