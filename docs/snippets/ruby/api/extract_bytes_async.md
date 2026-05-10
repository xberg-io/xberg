```ruby title="Ruby"
require 'kreuzberg'

pdf_bytes = File.read('document.pdf')
config = Kreuzberg::ExtractionConfig.new(
  enable_quality_processing: true
)

result = Kreuzberg.extract_bytes_async(
  pdf_bytes,
  'application/pdf',
  config: config
)

puts "Async bytes extraction done"
puts "Content preview: #{result.content[0..100]}"
puts "Quality score: #{result.quality_score}"
```
