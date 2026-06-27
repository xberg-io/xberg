```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  language_detection: Xberg::LanguageDetectionConfig.new(
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: true
  )
)

input = Xberg::ExtractInput.new(uri: 'multilingual_document.pdf')
result = Xberg.extract(input, config)

puts "Detected languages: #{result.results.first.detected_languages}"
# Output: ['eng', 'fra', 'deu']
```
