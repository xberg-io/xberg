```elixir title="Elixir"
alias Xberg.ExtractionConfig

config = %ExtractionConfig{
  ocr: %{"enabled" => true, "backend" => "tesseract"},
  chunking: %{"max_characters" => 1000, "overlap" => 100},
  language_detection: %{"enabled" => true},
  use_cache: true,
  force_ocr: false
}

{:ok, result} = Xberg.extract("document.pdf", nil, config)

IO.puts("Content length: #{byte_size(result.content)} characters")
IO.puts("Detected languages: #{inspect(result.detected_languages)}")
IO.puts("Chunks: #{if result.chunks, do: length(result.chunks), else: 0}")
```
