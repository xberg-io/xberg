```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  use_cache: true,
  enable_quality_processing: true
)

result = Kreuzberg.extract_file_sync('contract.pdf', config: config)

puts "Extracted #{result.content.length} characters"
puts "Quality score: #{result.quality_score}"
puts "Processing time: #{result.metadata&.dig('processing_time')}ms"
```
