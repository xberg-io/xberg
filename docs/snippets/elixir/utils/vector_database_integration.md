<!-- snippet:syntax-only -->

```elixir
config_json =
  Jason.encode!(%{
    "chunking" => %{
      "max_characters" => 512,
      "overlap" => 50,
      "embedding" => %{
        "model" => %{"preset" => %{"name" => "balanced"}},
        "normalize" => true
      }
    }
  })

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config_json)

result = List.first(output.results)
(result.chunks || [])
|> Enum.with_index()
|> Enum.each(fn {chunk, i} ->
  chunk_id = "doc_chunk_#{i}"
  preview = String.slice(chunk["content"] || "", 0, 50)
  IO.puts("Chunk #{chunk_id}: #{preview}")
end)
```
