```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  output_format: 'html',
  html_output: Kreuzberg::HtmlOutputConfig.new(
    theme: 'git_hub',
    embed_css: true
  )
)

result = Kreuzberg.extract_file_sync('document.pdf', nil, config)
puts result.content # HTML with kb-* classes
```
