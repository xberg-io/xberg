```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(backend: 'tesseract'),
  pdf: Kreuzberg::PdfConfig.new(dpi: 300)
)

result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
```
