```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng+deu+fra'
  )
)

result = Kreuzberg.extract_file_sync('multilingual.pdf', config: config)
puts result.content
```
