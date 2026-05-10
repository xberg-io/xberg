```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  chunking: Kreuzberg::ChunkingConfig.new(max_characters: 500, overlap: 50),
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

result.chunks&.each do |chunk|
  first = chunk.metadata.first_page
  last = chunk.metadata.last_page
  next if first.nil?

  page_range = first == last ? "Page #{first}" : "Pages #{first}-#{last}"
  puts "Chunk: #{chunk.content[0..50]}... (#{page_range})"
end
```
