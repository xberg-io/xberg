```ruby title="Ruby"
require "xberg"

inputs = [
  Xberg::ExtractInput.uri("document.pdf"),
  Xberg::ExtractInput.bytes("Hello from memory", "text/plain", "note.txt")
]

output = Xberg.extract_batch(inputs)

output.results.each do |result|
  puts result.content
end
```
