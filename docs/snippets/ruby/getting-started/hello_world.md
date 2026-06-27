```ruby title="Ruby"
require 'xberg'

input = Xberg::ExtractInput.new(uri: 'document.pdf')
config = Xberg::ExtractionConfig.new
result = Xberg.extract(input, config)
puts "Extracted content:"
puts result.results.first.content[0...200]
```
