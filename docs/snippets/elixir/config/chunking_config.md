```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "max_characters" => 1000,
    "overlap" => 200
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Chunks: #{length(result.chunks)}")

Enum.each(result.chunks, fn chunk ->
  IO.puts("Length: #{String.length(chunk.content)}")
end)
```
