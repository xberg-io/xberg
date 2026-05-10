```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  token_reduction: Kreuzberg::TokenReductionConfig.new(
    mode: 'moderate',
    preserve_important_words: true
  )
)
```
