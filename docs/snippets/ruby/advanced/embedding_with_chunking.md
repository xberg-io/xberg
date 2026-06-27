```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  chunking: Xberg::ChunkingConfig.new(
    max_characters: 512,
    overlap: 50,
    embedding: Xberg::EmbeddingConfig.new(
      model: Xberg::EmbeddingModelType.new(
        type: 'preset',
        name: 'balanced'
      ),
      normalize: true,
      batch_size: 32,
      show_download_progress: false
    )
  )
)

result = Xberg.extract_sync('document.pdf', config: config)

chunks = result.chunks || []
chunks.each_with_index do |chunk, idx|
  chunk_id = "doc_chunk_#{idx}"
  puts "Chunk #{chunk_id}: #{chunk.content[0...50]}"

  if chunk.embedding
    puts "  Embedding dimensions: #{chunk.embedding.length}"
  end
end
```
