```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(backend: 'tesseract'),
  pdf: Xberg::PdfConfig.new(dpi: 300)
)

input = Xberg::ExtractInput.new(uri: 'scanned.pdf')
result = Xberg.extract(input, config)
```
