```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  keywords: Xberg::KeywordConfig.new(
    algorithm: Xberg::KeywordAlgorithm::YAKE,
    max_keywords: 10,
    min_score: 0.3
  )
)

output = Xberg.extract('research_paper.pdf', config: config)
result = output.results.first

keywords = result.extracted_keywords
keywords.each do |kw|
  puts "#{kw.text}: #{kw.score.round(3)}"
end
```
