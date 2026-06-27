# Advanced Chunking Configuration
# This example shows how to configure sophisticated document chunking strategies
# with fine-grained control over chunk size, overlap, and boundary respect.

alias Xberg.ExtractionConfig

# Advanced chunking configuration with multiple parameters
config = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_characters" => 1000,
    "overlap" => 200,
    "min_size" => 100,
    "respect_boundaries" => true,
    "split_on" => ["sentence", "paragraph"]
  }
}

# Use the configuration for extraction
{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config)
result = List.first(output.results)
# Process the chunked results
chunks = result.chunks || []

IO.puts("Total chunks: #{length(chunks)}")

Enum.each(chunks, fn chunk ->
  IO.inspect(%{
    text_length: String.length(chunk["content"]),
    page: chunk["page"],
    boundaries_respected: !String.ends_with?(chunk["content"], [" ", "\n"])
  })
end)

# Prepend heading context to chunk content
config_with_headings = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "chunker_type" => "markdown",
    "prepend_heading_context" => true
  }
}

{:ok, output_headings} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.md"}, config_with_headings)

result_headings = List.first(output_headings.results)
if result_headings.chunks do
  IO.puts("Generated #{length(result_headings.chunks)} chunks with prepended headings")

  Enum.each(result_headings.chunks, fn chunk ->
    # Each chunk's content is prefixed with its heading breadcrumb
    IO.puts("Chunk preview: #{String.slice(chunk["content"], 0..80)}...")
  end)
end
