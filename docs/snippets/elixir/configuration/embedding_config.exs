```elixir title="Elixir"
alias Xberg.ExtractionConfig

# Configure embeddings for vector search
config = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_characters" => 512,
    "overlap" => 50
  },
  embeddings: %{
    "enabled" => true,
    "model" => "sentence-transformers/all-MiniLM-L6-v2"
  }
}

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config)

result = List.first(output.results)
IO.puts("Extracted chunks with embeddings: #{length(result.chunks || [])}")
```
