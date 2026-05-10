```ruby title="Ruby"
require 'kreuzberg'
require 'json'

schema = {
  type: 'object',
  properties: {
    title: { type: 'string' },
    authors: { type: 'array', items: { type: 'string' } },
    date: { type: 'string' }
  },
  required: %w[title authors date],
  additionalProperties: false
}

config = Kreuzberg::ExtractionConfig.new(
  structured_extraction: Kreuzberg::StructuredExtractionConfig.new(
    schema: JSON.generate(schema),
    schema_name: 'PaperMetadata',
    strict: true,
    llm: Kreuzberg::LlmConfig.new(model: 'openai/gpt-4o-mini')
  )
)

result = Kreuzberg.extract_file_sync('paper.pdf', config: config)
puts result.structured_output
```
