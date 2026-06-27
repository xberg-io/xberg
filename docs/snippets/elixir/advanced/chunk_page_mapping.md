```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "enabled" => true,
    "max_characters" => 1024,
    "overlap" => 128
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
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
