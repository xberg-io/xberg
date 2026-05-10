```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  use_cache: false,
  enable_quality_processing: true
)

result = Kreuzberg.extract_file_async('document.pdf', config: config)

puts "Async extraction complete"
puts "Extracted #{result.content.length} characters"
puts "Quality: #{result.quality_score}"
```
