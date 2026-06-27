```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  language_detection: Xberg::LanguageDetectionConfig.new(
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: true
  )
)

result = Xberg.extract_sync('multilingual_document.pdf', config: config)

languages = result.detected_languages || []

if languages.any?
  puts "Detected #{languages.length} language(s): #{languages.join(', ')}"
else
  puts "No languages detected"
end

puts "Total content: #{result.content.length} characters"
puts "MIME type: #{result.mime_type}"
```
