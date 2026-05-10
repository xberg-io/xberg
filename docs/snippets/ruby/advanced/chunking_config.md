```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  chunking: Kreuzberg::ChunkingConfig.new(
    max_characters: 1000,
    overlap: 200,
    embedding: Kreuzberg::EmbeddingConfig.new(
      model: Kreuzberg::EmbeddingModelType.new(
        type: 'preset',
        name: 'all-minilm-l6-v2'
      ),
      normalize: true,
      batch_size: 32
    )
  )
)
```

```ruby title="Ruby - Prepend Heading Context"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  chunking: Kreuzberg::ChunkingConfig.new(
    chunker_type: "markdown",
    max_characters: 500,
    overlap: 50,
    prepend_heading_context: true
  )
)
```
