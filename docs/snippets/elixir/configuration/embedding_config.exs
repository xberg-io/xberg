```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Configure embeddings for vector search
config = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_chars" => 512,
    "max_overlap" => 50
  },
  embeddings: %{
    "enabled" => true,
    "model" => "sentence-transformers/all-MiniLM-L6-v2"
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

IO.puts("Extracted chunks with embeddings: #{length(result.chunks || [])}")
```
