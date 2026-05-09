```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  language_detection: Kreuzberg::LanguageDetectionConfig.new(
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: false
  )
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

if result.detected_languages&.any?
  puts "Detected Language: #{result.detected_languages.first}"
else
  puts "No language detected"
end

puts "Content length: #{result.content.length} characters"
```
