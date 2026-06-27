```elixir title="Elixir"
alias Xberg.ExtractionConfig

# Configure PDF-specific extraction options
# Extract annotations, metadata, and flatten forms for comprehensive document processing
config = %ExtractionConfig{
  pdf: %{
    "extract_annotations" => true,
    "extract_metadata" => true,
    "flatten_forms" => true
  },
  chunking: %{
    "max_characters" => 1500,
    "overlap" => 150
  },
  use_cache: true
}

{:ok, result} = Xberg.extract("form_document.pdf", nil, config)

IO.puts("PDF Extraction Complete:")
IO.puts("Content length: #{byte_size(result.content)} bytes")
IO.puts("Metadata: #{inspect(result.metadata)}")
IO.puts("Annotations present: #{map_size(result.metadata["annotations"] || %{}) > 0}")
```
