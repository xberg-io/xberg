```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Configure post-processing pipeline
# Applies transformations to extracted content after initial processing
config = %ExtractionConfig{
  postprocessing: %{
    "remove_whitespace" => true,
    "normalize_unicode" => true,
    "fix_encoding" => true
  },
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract"
  },
  use_cache: true,
  force_ocr: false
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

IO.puts("Post-Processing Configuration Applied:")
IO.puts("Remove Whitespace: true")
IO.puts("Normalize Unicode: true")
IO.puts("Fix Encoding: true")
IO.puts("Original content length: #{byte_size(result.content)} bytes")
IO.puts("Processed content: #{String.slice(result.content, 0..100)}...")
IO.puts("Metadata: #{inspect(result.metadata)}")
```
