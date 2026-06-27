```ruby title="Ruby"
require "xberg"

config = Xberg::ExtractionConfig.new(
  use_cache: true,
  ocr: Xberg::OcrConfig.new(
    backend: "tesseract",
    language: "eng+deu",
    tesseract: Xberg::TesseractConfig.new(psm: 6)
  ),
  chunking: Xberg::ChunkingConfig.new(
    max_characters: 1000,
    overlap: 200
  )
)

result = Xberg.extract_sync("document.pdf", config)
puts "Content length: #{result.content.length}"
```
