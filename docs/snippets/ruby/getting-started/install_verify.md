```ruby title="Ruby"
require 'xberg'

puts "Xberg version: #{Xberg::VERSION}"
puts "FFI bindings loaded successfully"

result = Xberg.extract_sync('sample.pdf')
puts "Installation verified! Extracted #{result.content.length} characters"
```
