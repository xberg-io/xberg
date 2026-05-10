```ruby title="Ruby"
require 'kreuzberg'

items = [
  Kreuzberg::BatchBytesItem.new(
    content: File.read('doc1.pdf'),
    mime_type: 'application/pdf'
  ),
  Kreuzberg::BatchBytesItem.new(
    content: File.read('doc2.docx'),
    mime_type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
  ),
  Kreuzberg::BatchBytesItem.new(
    content: File.read('doc3.xlsx'),
    mime_type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
  )
]

config = Kreuzberg::ExtractionConfig.new(use_cache: true)

results = Kreuzberg.batch_extract_bytes_sync(items, config: config)

results.each { |result| puts "Extracted: #{result.content.length} chars" }
```
