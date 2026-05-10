```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  use_cache: true,
  enable_quality_processing: true
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

puts "Extracted #{result.content.length} characters"
puts "MIME type: #{result.mime_type}"
puts "Quality score: #{result.quality_score}"
```
