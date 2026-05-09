```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng+deu'
  ),
  chunking: Kreuzberg::ChunkingConfig.new(
    max_characters: 1000,
    overlap: 100
  ),
  language_detection: Kreuzberg::LanguageDetectionConfig.new,
  use_cache: true,
  enable_quality_processing: true
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

result.chunks&.each { |chunk| puts chunk[0..100] }
puts "Languages: #{result.detected_languages.inspect}"
```
