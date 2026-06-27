```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "enabled" => true,
    "max_characters" => 1000,
    "overlap" => 200,
    "min_size" => 100,
    "respect_boundaries" => true,
    "split_on" => ["sentence", "paragraph"]
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)

chunks = result.chunks || []
IO.puts("Total chunks: #{length(chunks)}")

Enum.each(chunks, fn chunk ->
  IO.inspect(%{
    text_length: String.length(chunk["content"]),
    page: chunk["page"],
    boundaries_respected: !String.ends_with?(chunk["content"], [" ", "\n"])
  })
end)
```

```elixir title="Elixir - Prepend Heading Context"
config_json = Jason.encode!(%{
  "chunking" => %{
    "enabled" => true,
    "chunker_type" => "markdown",
    "prepend_heading_context" => true
  }
})

{:ok, result} = Xberg.extract_sync("document.md", "text/markdown", config_json)

if result.chunks do
  IO.puts("Generated #{length(result.chunks)} chunks with prepended headings")

  Enum.each(result.chunks, fn chunk ->
    IO.puts("Chunk preview: #{String.slice(chunk["content"], 0..80)}...")
  end)
end
```
