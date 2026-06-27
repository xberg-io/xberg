```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  use_cache: true,
  enable_quality_processing: true
)

input = Xberg::ExtractInput.new(uri: 'document.pdf')
result = Xberg.extract(input, config)
```
