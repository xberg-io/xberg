```elixir title="Elixir"
alias Xberg.ExtractionConfig

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

{:ok, result} = Xberg.extract("document.pdf", nil, config)

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

{:ok, result2} = Xberg.extract("document.md", nil, config2)

if result2.chunks do
  IO.puts("Generated #{length(result2.chunks)} markdown chunks")

  Enum.each(result2.chunks, fn chunk ->
    IO.puts("\nChunk preview: #{String.slice(chunk["content"], 0..60)}...")

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

# Example 3: Prepend heading context to chunk content
config3 = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "chunker_type" => "markdown",
    "prepend_heading_context" => true
  }
}

{:ok, result3} = Xberg.extract("document.md", nil, config3)

if result3.chunks do
  IO.puts("Generated #{length(result3.chunks)} chunks with prepended headings")

  Enum.each(result3.chunks, fn chunk ->
    # Each chunk's content is prefixed with its heading breadcrumb
    IO.puts("\nChunk preview: #{String.slice(chunk["content"], 0..80)}...")
  end)
end
```
