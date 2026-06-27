```elixir title="Elixir"
alias Xberg.ExtractionConfig

config = %ExtractionConfig{
  ocr: %{"enabled" => true, "backend" => "paddle-ocr", "language" => "en"}
  # Add "model_tier" => "server" for max accuracy
}

{:ok, result} = Xberg.extract("scanned_document.pdf", nil, config)

IO.puts("OCR Extracted content:")
IO.puts(result.content)
```
