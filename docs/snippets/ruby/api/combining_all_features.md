```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  enable_quality_processing: true,
  
  language_detection: Kreuzberg::LanguageDetectionConfig.new(
    enabled: true,
    detect_multiple: true,
    min_confidence: 0.8
  ),
  
  token_reduction: Kreuzberg::TokenReductionOptions.new(
    mode: 'moderate',
    preserve_important_words: true
  ),
  
  chunking: Kreuzberg::ChunkingConfig.new(
    max_characters: 512,
    overlap: 50,
    embedding: Kreuzberg::EmbeddingConfig.new(
      model: { type: 'preset', name: 'text-embedding-all-minilm-l6-v2' }
    )
  ),
  
  keywords: Kreuzberg::KeywordConfig.new(
    algorithm: 'yake',
    max_keywords: 10
  )
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

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
