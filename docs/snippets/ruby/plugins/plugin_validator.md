```ruby title="Ruby"
require "xberg"

validator = lambda do |result|
  raise StandardError, "Content too short" if result.content.length < 50
end

Xberg.register_validator("min_length", validator, priority: 10)

result = Xberg.extract_sync("document.pdf")
puts "Validated content length: #{result.content.length}"

Xberg.unregister_validator("min_length")
```
