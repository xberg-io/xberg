```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  enable_quality_processing: true,

  language_detection: Xberg::LanguageDetectionConfig.new(
    enabled: true,
    detect_multiple: true,
    min_confidence: 0.8
  ),

  token_reduction: Xberg::TokenReductionOptions.new(
    mode: 'moderate',
    preserve_important_words: true
  ),

  chunking: Xberg::ChunkingConfig.new(
    max_characters: 512,
    overlap: 50,
    embedding: Xberg::EmbeddingConfig.new(
      model: { type: 'preset', name: 'text-embedding-all-minilm-l6-v2' }
    )
  ),

  keywords: Xberg::KeywordConfig.new(
    algorithm: 'yake',
    max_keywords: 10
  )
)

result = Xberg.extract_sync('document.pdf', config: config)

puts "Content length: #{result.content.length} characters"
puts "Quality score: #{result.quality_score}"
puts "Detected languages: #{result.detected_languages&.join(', ')}"
puts "Total chunks: #{result.chunks&.length || 0}"
puts "Keywords: #{result.extracted_keywords&.map(&:text)&.join(', ')}"

if result.chunks && result.chunks.length > 0
  first_chunk = result.chunks[0]
  puts "First chunk size: #{first_chunk.content.length} chars"
  puts "Embedding dims: #{first_chunk.embedding&.length || 0}"
end
```
