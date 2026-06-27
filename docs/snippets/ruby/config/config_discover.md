```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.discover
input = Xberg::ExtractInput.new(uri: 'document.pdf')
result = Xberg.extract(input, config)
```
