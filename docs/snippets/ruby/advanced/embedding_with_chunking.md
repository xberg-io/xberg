```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  chunking: Kreuzberg::ChunkingConfig.new(
    max_characters: 512,
    overlap: 50,
    embedding: Kreuzberg::EmbeddingConfig.new(
      model: Kreuzberg::EmbeddingModelType.new(
        type: 'preset',
        name: 'balanced'
      ),
      normalize: true,
      batch_size: 32,
      show_download_progress: false
    )
  )
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

chunks = result.chunks || []
chunks.each_with_index do |chunk, idx|
  chunk_id = "doc_chunk_#{idx}"
  puts "Chunk #{chunk_id}: #{chunk.content[0...50]}"

  if chunk.embedding
    puts "  Embedding dimensions: #{chunk.embedding.length}"
  end
end
```
