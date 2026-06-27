```elixir title="Elixir"
alias Xberg.ExtractionConfig

config = %ExtractionConfig{
  ocr: %{"enabled" => true, "backend" => "tesseract"}
}

{:ok, result} = Xberg.extract("scanned_document.pdf", nil, config)

content = result.content
IO.puts("OCR Extracted content:")
IO.puts(content)
IO.puts("Metadata: #{inspect(result.metadata)}")
```
