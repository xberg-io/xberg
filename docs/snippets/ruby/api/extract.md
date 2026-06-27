```ruby title="Ruby"
require "xberg"

output = Xberg.extract(Xberg::ExtractInput.uri("document.pdf"))

puts output.results.first.content
puts "Results: #{output.summary.results}"
```
