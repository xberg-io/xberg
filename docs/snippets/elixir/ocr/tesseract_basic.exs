```elixir title="Elixir"
alias Xberg.ExtractionConfig

config = %ExtractionConfig{
  ocr: %{"enabled" => true, "backend" => "tesseract"}
}

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "scanned_document.pdf"}, config)

result = List.first(output.results)
content = result.content
IO.puts("OCR Extracted content:")
IO.puts(content)
IO.puts("Metadata: #{inspect(result.metadata)}")
```
