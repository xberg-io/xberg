```ruby title="Ruby"
require 'xberg'

result = Xberg.extract_sync('document.pdf')
puts "Extracted content:"
puts result.content[0...200]
```
