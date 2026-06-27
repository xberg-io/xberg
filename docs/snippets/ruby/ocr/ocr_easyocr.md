```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(
    backend: 'easyocr',
    language: 'eng'
  )
)

result = Xberg.extract_sync('scanned.pdf', config: config)
puts result.content[0..100]
puts "Total length: #{result.content.length}"
```
