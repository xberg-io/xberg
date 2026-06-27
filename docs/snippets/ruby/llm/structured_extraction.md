```ruby title="Ruby"
require 'xberg'
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

config = Xberg::ExtractionConfig.new(
  structured_extraction: Xberg::StructuredExtractionConfig.new(
    schema: JSON.generate(schema),
    schema_name: 'PaperMetadata',
    strict: true,
    llm: Xberg::LlmConfig.new(model: 'openai/gpt-4o-mini')
  )
)

result = Xberg.extract_sync('paper.pdf', config: config)
puts result.structured_output
```
