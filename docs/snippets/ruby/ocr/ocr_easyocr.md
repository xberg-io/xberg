```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(
    backend: 'easyocr',
    language: 'eng'
  )
)

result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
puts result.content[0..100]
puts "Total length: #{result.content.length}"
```
