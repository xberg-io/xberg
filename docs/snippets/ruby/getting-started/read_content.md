```ruby title="Ruby"
require 'xberg'

input = Xberg::ExtractInput.new(uri: 'document.pdf')
config = Xberg::ExtractionConfig.new
result = Xberg.extract(input, config)

content = result.results.first.content
tables = result.results.first.tables
images = result.results.first.images
metadata = result.results.first.metadata

puts "Content: #{content.length} characters"
puts "Tables: #{tables.length}"
puts "Images: #{images.length}"
puts "Metadata keys: #{metadata&.keys&.join(', ')}"
```
