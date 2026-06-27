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
      normalize: true
    )
  )
)

result = Xberg.extract_sync('document.pdf', config: config)

result.chunks.each_with_index do |chunk, i|
  if chunk.embedding
    puts "Chunk #{i}: #{chunk.embedding.length} dimensions"
    # Store in vector database
  end
end
```
