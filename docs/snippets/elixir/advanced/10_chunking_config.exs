# Advanced Chunking Configuration
# This example shows how to configure sophisticated document chunking strategies
# with fine-grained control over chunk size, overlap, and boundary respect.

alias Kreuzberg.ExtractionConfig

# Advanced chunking configuration with multiple parameters
config = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_chars" => 1000,
    "max_overlap" => 200,
    "min_size" => 100,
    "respect_boundaries" => true,
    "split_on" => ["sentence", "paragraph"]
  }
}

# Use the configuration for extraction
{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Process the chunked results
chunks = result.chunks || []

IO.puts("Total chunks: #{length(chunks)}")

Enum.each(chunks, fn chunk ->
  IO.inspect(%{
    text_length: String.length(chunk["text"]),
    page: chunk["page"],
    boundaries_respected: !String.ends_with?(chunk["text"], [" ", "\n"])
  })
end)
