```ruby title="Ruby"
require 'xberg'

ocr_config = Xberg::OcrConfig.new(
  backend: 'tesseract',
  language: 'eng'
)

config = Xberg::ExtractionConfig.new(ocr: ocr_config)
result = Xberg.extract_sync('scanned.pdf', config: config)
puts result.content
```
