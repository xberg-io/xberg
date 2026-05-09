```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(backend: 'tesseract'),
  force_ocr: true
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)
puts result.content
```
