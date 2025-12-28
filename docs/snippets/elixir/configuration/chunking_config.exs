```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Configure chunking for RAG applications
config = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_chars" => 1000,
    "max_overlap" => 100,
    "min_size" => 200,
    "respect_boundaries" => true
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Access chunks
if result.chunks do
  IO.puts("Generated #{length(result.chunks)} chunks")

  Enum.each(result.chunks, fn chunk ->
    IO.puts("Chunk: #{String.slice(chunk["text"], 0..50)}...")
  end)
end
```
