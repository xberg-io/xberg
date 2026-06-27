```elixir title="Elixir"
alias Xberg.ExtractionConfig

config = %ExtractionConfig{
  ocr: %{"enabled" => true, "backend" => "paddle-ocr", "language" => "en"}
  # Add "model_tier" => "server" for max accuracy
}

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "scanned_document.pdf"}, config)

result = List.first(output.results)
IO.puts("OCR Extracted content:")
IO.puts(result.content)
```
