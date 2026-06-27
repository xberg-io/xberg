```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "max_characters" => 1000,
    "overlap" => 200
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts("Chunks: #{length(result.chunks)}")

Enum.each(result.chunks, fn chunk ->
  IO.puts("Length: #{String.length(chunk.content)}")
end)
```
