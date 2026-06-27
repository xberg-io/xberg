```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  output_format: 'html',
  html_output: Xberg::HtmlOutputConfig.new(
    theme: 'git_hub',
    embed_css: true
  )
)

input = Xberg::ExtractInput.new(uri: 'document.pdf')
result = Xberg.extract(input, config)
puts result.results.first.content # HTML with kb-* classes
```
