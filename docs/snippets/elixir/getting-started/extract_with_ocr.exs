```elixir title="Elixir"
# Extract scanned documents with OCR
# Configure Tesseract for OCR processing

ocr_config = %Xberg.Config.OCR{
  backend: "tesseract",
  language: "eng"
}

config = %Xberg.Config.Extraction{
  ocr: ocr_config
}

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "scanned.pdf"}, config)

result = List.first(output.results)
IO.puts("Extracted text from scanned document:")
IO.puts(result.content)
IO.puts("Used OCR backend: tesseract")
```
