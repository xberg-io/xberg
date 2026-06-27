```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(backend: 'tesseract'),
  pdf: Xberg::PdfConfig.new(dpi: 300)
)

result = Xberg.extract_sync('scanned.pdf', config: config)
```
