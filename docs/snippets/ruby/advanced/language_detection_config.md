```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  language_detection: Xberg::LanguageDetectionConfig.new(
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: false
  )
)

result = Xberg.extract_sync('document.pdf', config: config)

if result.detected_languages&.any?
  puts "Detected Language: #{result.detected_languages.first}"
else
  puts "No language detected"
end

puts "Content length: #{result.content.length} characters"
```
