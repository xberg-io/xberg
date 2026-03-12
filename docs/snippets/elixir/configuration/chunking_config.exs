```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Example 1: Basic character-based chunking for RAG applications
config = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_characters" => 1000,
    "overlap" => 100,
    "min_size" => 200,
    "respect_boundaries" => true
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Access chunks
if result.chunks do
  IO.puts("Generated #{length(result.chunks)} chunks")

  Enum.each(result.chunks, fn chunk ->
    IO.puts("Chunk: #{String.slice(chunk["content"], 0..50)}...")
  end)
end

# Example 2: Markdown chunker with token-based sizing and heading context
config2 = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "chunker_type" => "markdown",
    "sizing" => %{
      "type" => "tokenizer",
      "model" => "Xenova/gpt-4o"
    }
  }
}

{:ok, result2} = Kreuzberg.extract_file("document.md", nil, config2)

if result2.chunks do
  IO.puts("Generated #{length(result2.chunks)} markdown chunks")

  Enum.each(result2.chunks, fn chunk ->
    IO.puts("\nChunk preview: #{String.slice(chunk["text"], 0..60)}...")

    # Access heading context
    if is_map(chunk["metadata"]) and is_map(chunk["metadata"]["heading_context"]) do
      headings = chunk["metadata"]["heading_context"]["headings"] || []
      if length(headings) > 0 do
        IO.puts("  Headings in context:")
        Enum.each(headings, fn heading ->
          IO.puts("    - Level #{heading["level"]}: #{heading["text"]}")
        end)
      end
    end
  end)
end
```
