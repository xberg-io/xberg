```ruby title="Ruby"
require 'kreuzberg'

pdf_bytes = File.read('document.pdf')
config = Kreuzberg::ExtractionConfig.new(
  use_cache: true
)

result = Kreuzberg.extract_bytes_sync(
  pdf_bytes,
  'application/pdf',
  config: config
)

puts "Extracted #{result.content.length} characters"
puts "Detected MIME: #{result.mime_type}"
```
