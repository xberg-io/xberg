```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Extract text from a scanned PDF using OCR
# Tesseract processes the document and returns structured content

config = %ExtractionConfig{
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract"
  },
  chunking: %{
    "max_chars" => 1500,
    "max_overlap" => 150
  },
  language_detection: %{"enabled" => true},
  use_cache: true
}

{:ok, result} = Kreuzberg.extract_file("scanned_invoice.pdf", nil, config)

# Process the extracted content
content = result.content
chunks = result.chunks || []
metadata = result.metadata || %{}

IO.puts("OCR Extraction Complete:")
IO.puts("Content length: #{byte_size(content)} bytes")
IO.puts("Number of chunks: #{length(chunks)}")
IO.puts("Detected languages: #{inspect(result.detected_languages)}")
IO.puts("Creation date: #{metadata["creation_date"] || "N/A"}")
IO.puts("\nFirst 200 characters of extracted text:")
IO.puts(String.slice(content, 0..199))
```
