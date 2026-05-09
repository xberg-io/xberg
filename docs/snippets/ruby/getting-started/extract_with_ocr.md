```ruby title="Ruby"
require 'kreuzberg'

ocr_config = Kreuzberg::OcrConfig.new(
  backend: 'tesseract',
  language: 'eng'
)

config = Kreuzberg::ExtractionConfig.new(ocr: ocr_config)
result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)

puts "Extracted text from scanned document:"
puts result.content
puts "Used OCR backend: tesseract"
```
