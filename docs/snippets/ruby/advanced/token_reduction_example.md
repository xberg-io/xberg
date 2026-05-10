```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  token_reduction: Kreuzberg::TokenReductionConfig.new(
    mode: 'moderate',
    preserve_markdown: true
  )
)

result = Kreuzberg.extract_file_sync('verbose_document.pdf', config: config)

original_tokens = result.metadata&.dig('original_token_count') || 0
reduced_tokens = result.metadata&.dig('token_count') || 0
reduction_ratio = result.metadata&.dig('token_reduction_ratio') || 0.0

puts "Reduced from #{original_tokens} to #{reduced_tokens} tokens"
puts "Reduction: #{(reduction_ratio * 100).round(1)}%"
```
