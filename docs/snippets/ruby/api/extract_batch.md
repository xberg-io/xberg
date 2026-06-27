```ruby title="Ruby"
require "xberg"

inputs = [
  Xberg::ExtractInput.new(kind: Xberg::ExtractInputKind::Uri, uri: "document.pdf"),
  Xberg::ExtractInput.new(
    kind: Xberg::ExtractInputKind::Bytes,
    bytes: "Hello from memory",
    mime_type: "text/plain",
    filename: "note.txt"
  )
]

output = Xberg.extract_batch(inputs)

output.results.each do |result|
  puts result.content
end
```
