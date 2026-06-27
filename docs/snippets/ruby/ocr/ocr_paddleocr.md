```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(
    backend: 'paddleocr',
    language: 'eng'
    # model_tier: 'server' # for max accuracy
  )
)

result = Xberg.extract_sync('scanned.pdf', config: config)
puts result.content[0..100]
puts "Total length: #{result.content.length}"
```
