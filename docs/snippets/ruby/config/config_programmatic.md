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

input = Xberg::ExtractInput.new(uri: "document.pdf")
result = Xberg.extract(input, config)
puts "Content length: #{result.results.first.content.length}"
```
