```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng+deu+fra'
  )
)

result = Xberg.extract_sync('multilingual.pdf', config: config)
puts result.content
```
