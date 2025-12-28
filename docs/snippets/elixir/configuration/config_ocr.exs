```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Configure OCR-specific settings
# Includes backend selection, language settings, and preprocessing options
config = %ExtractionConfig{
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract",
    "language" => "eng",
    "preprocessing" => true
  },
  chunking: %{
    "max_chars" => 2000,
    "max_overlap" => 200
  },
  use_cache: true,
  force_ocr: false
}

{:ok, result} = Kreuzberg.extract_file("scanned_document.pdf", nil, config)

IO.puts("OCR Configuration Applied:")
IO.puts("Backend: tesseract")
IO.puts("Language: eng")
IO.puts("Content extracted: #{byte_size(result.content)} bytes")
IO.puts("Metadata: #{inspect(result.metadata)}")
```
