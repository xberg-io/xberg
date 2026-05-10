```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.discover
result = Kreuzberg.extract_file_sync('document.pdf', config: config)
```
