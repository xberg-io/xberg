```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "enabled" => true,
    "max_characters" => 512,
    "overlap" => 50,
    "respect_boundaries" => true
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
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
