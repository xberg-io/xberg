```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng+deu'
  ),
  chunking: Xberg::ChunkingConfig.new(
    max_characters: 1000,
    overlap: 100
  ),
  language_detection: Xberg::LanguageDetectionConfig.new,
  use_cache: true,
  enable_quality_processing: true
)

input = Xberg::ExtractInput.new(uri: 'document.pdf')
result = Xberg.extract(input, config)

result.results.first.chunks&.each { |chunk| puts chunk[0..100] }
puts "Languages: #{result.results.first.detected_languages.inspect}"
```
