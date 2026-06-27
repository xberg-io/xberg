```ruby title="Document Structure Config (Ruby)"
require 'xberg'

config = Xberg::ExtractionConfig.new(include_document_structure: true)

input = Xberg::ExtractInput.new(uri: 'document.pdf')
result = Xberg.extract(input, config)

if result.results.first.document
  result.results.first.document['nodes'].each do |node|
    node_type = node['content']['node_type']
    text = node['content']['text'] || ''
    puts "[#{node_type}] #{text[0...80]}"
  end
end
```
