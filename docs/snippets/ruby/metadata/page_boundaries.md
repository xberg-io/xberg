```ruby title="Ruby"
require 'xberg'

input = Xberg::ExtractInput.new(uri: 'document.pdf')
config = Xberg::ExtractionConfig.new
result = Xberg.extract(input, config)
first_result = result.results.first

if first_result.metadata.pages&.boundaries
  content_bytes = first_result.content.bytes

  first_result.metadata.pages.boundaries.take(3).each do |boundary|
    page_bytes = content_bytes[boundary.byte_start...boundary.byte_end]
    page_text = page_bytes.pack('C*').force_encoding('UTF-8')

    puts "Page #{boundary.page_number}:"
    puts "  Byte range: #{boundary.byte_start}-#{boundary.byte_end}"
    puts "  Preview: #{page_text[0..100]}..."
  end
end
```
