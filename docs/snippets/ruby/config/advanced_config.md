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

result = Xberg.extract_sync('document.pdf', config: config)

result.chunks&.each { |chunk| puts chunk[0..100] }
puts "Languages: #{result.detected_languages.inspect}"
```
