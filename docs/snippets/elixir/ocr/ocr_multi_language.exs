```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Perform OCR extraction with multiple languages
# Tesseract can recognize text in multiple languages simultaneously
# Combine language codes with '+' separator: "eng+fra+deu" for English, French, German

config = %ExtractionConfig{
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract",
    "language" => "eng+fra+deu"
  },
  chunking: %{
    "max_chars" => 2000,
    "max_overlap" => 200
  },
  language_detection: %{"enabled" => true},
  use_cache: true,
  force_ocr: true
}

{:ok, result} = Kreuzberg.extract_file("multilingual_document.pdf", nil, config)

# Results will contain text recognized in all specified languages
IO.puts("Multi-language OCR Extraction:")
IO.puts("Supported languages: English, French, German")
IO.puts("Content extracted: #{byte_size(result.content)} bytes")
IO.puts("Detected languages: #{inspect(result.detected_languages)}")
IO.puts("Chunks created: #{if result.chunks, do: length(result.chunks), else: 0}")
IO.puts("\nExtracted content preview:")
IO.puts(String.slice(result.content, 0..250))

# Access metadata if available
metadata = result.metadata || %{}
IO.puts("\nMetadata:")
IO.puts("Pages: #{metadata["pages"] || "Unknown"}")
IO.puts("Format: #{metadata["format"] || "Unknown"}")
```
