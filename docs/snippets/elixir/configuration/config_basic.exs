```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

config = %ExtractionConfig{
  ocr: %{"enabled" => true, "backend" => "tesseract"},
  chunking: %{"max_chars" => 1000, "max_overlap" => 100},
  language_detection: %{"enabled" => true},
  use_cache: true,
  force_ocr: false
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

IO.puts("Content length: #{byte_size(result.content)} characters")
IO.puts("Detected languages: #{inspect(result.detected_languages)}")
IO.puts("Chunks: #{if result.chunks, do: length(result.chunks), else: 0}")
```
