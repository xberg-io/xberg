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

input = Xberg::ExtractInput.new(uri: 'document.pdf')
result = Xberg.extract(input, config)
first_result = result.results.first

puts "Content length: #{first_result.content.length} characters"
puts "Quality score: #{first_result.quality_score}"
puts "Detected languages: #{first_result.detected_languages&.join(', ')}"
puts "Total chunks: #{first_result.chunks&.length || 0}"
puts "Keywords: #{first_result.extracted_keywords&.map(&:text)&.join(', ')}"

if first_result.chunks && first_result.chunks.length > 0
  first_chunk = first_result.chunks[0]
  puts "First chunk size: #{first_chunk.content.length} chars"
  puts "Embedding dims: #{first_chunk.embedding&.length || 0}"
end
```
