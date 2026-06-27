```ruby title="Ruby"
require 'xberg'

config = Xberg::ExtractionConfig.new(
  output_format: 'html',
  html_output: Xberg::HtmlOutputConfig.new(
    theme: 'git_hub',
    embed_css: true
  )
)

result = Xberg.extract_sync('document.pdf', nil, config)
puts result.content # HTML with kb-* classes
```
