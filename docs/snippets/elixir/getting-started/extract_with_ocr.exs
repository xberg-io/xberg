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

{:ok, result} = Xberg.extract("scanned.pdf", config: config)

IO.puts("Extracted text from scanned document:")
IO.puts(result.content)
IO.puts("Used OCR backend: tesseract")
```
