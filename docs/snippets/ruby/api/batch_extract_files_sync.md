```ruby title="Ruby"
require 'kreuzberg'

items = [
  Kreuzberg::BatchFileItem.new(path: 'doc1.pdf'),
  Kreuzberg::BatchFileItem.new(path: 'doc2.docx'),
  Kreuzberg::BatchFileItem.new(path: 'doc3.pptx')
]

config = Kreuzberg::ExtractionConfig.new(use_cache: true)

results = Kreuzberg.batch_extract_files_sync(items, config: config)

results.each_with_index do |result, idx|
  puts "Document #{idx + 1}:"
  puts "  Extracted: #{result.content.length} characters"
  puts "  Quality: #{result.quality_score}"
  puts "  MIME: #{result.mime_type}"
end
```
