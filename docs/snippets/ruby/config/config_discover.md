```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.discover
result = Xberg.extract_sync('document.pdf', config: config)
```
