```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  use_cache: true,
  enable_quality_processing: true
)

result = Xberg.extract_sync('document.pdf', config: config)
```
