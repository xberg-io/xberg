```ruby title="Document Structure Config (Ruby)"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(include_document_structure: true)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

if result.document
  result.document['nodes'].each do |node|
    node_type = node['content']['node_type']
    text = node['content']['text'] || ''
    puts "[#{node_type}] #{text[0...80]}"
  end
end
```
