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

puts "Detected languages: #{result.detected_languages}"
# Output: ['eng', 'fra', 'deu']
```
