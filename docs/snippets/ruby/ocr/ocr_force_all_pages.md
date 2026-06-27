```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(backend: 'tesseract'),
  force_ocr: true
)

result = Xberg.extract_sync('document.pdf', config: config)
puts result.content
```
