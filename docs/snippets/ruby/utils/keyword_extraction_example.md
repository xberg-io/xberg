```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  keywords: Kreuzberg::KeywordConfig.new(
    algorithm: Kreuzberg::KeywordAlgorithm::YAKE,
    max_keywords: 10,
    min_score: 0.3
  )
)

result = Kreuzberg.extract_file_sync('research_paper.pdf', config: config)

keywords = result.extracted_keywords
keywords.each do |kw|
  puts "#{kw['text']}: #{kw['score'].round(3)}"
end
```
