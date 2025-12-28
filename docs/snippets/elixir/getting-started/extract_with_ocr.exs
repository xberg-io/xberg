```elixir title="Elixir"
# Extract scanned documents with OCR
# Configure Tesseract for OCR processing

ocr_config = %Kreuzberg.Config.OCR{
  backend: "tesseract",
  language: "eng"
}

config = %Kreuzberg.Config.Extraction{
  ocr: ocr_config
}

{:ok, result} = Kreuzberg.extract_file("scanned.pdf", config: config)

IO.puts("Extracted text from scanned document:")
IO.puts(result.content)
IO.puts("Used OCR backend: tesseract")
```
