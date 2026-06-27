```ruby title="Ruby"
require 'xberg'

class PdfMetadataExtractor
  def initialize
    @count = 0
  end

  def call(result)
    return result unless result['mime_type'] == 'application/pdf'
    @count += 1
    result['metadata'] ||= {}
    result['metadata']['pdf_order'] = @count
    result
  end
end

extractor = PdfMetadataExtractor.new
Xberg.register_post_processor('pdf_metadata', extractor)

config = Xberg::ExtractionConfig.new(
  postprocessor: { enabled: true }
)

result = Xberg.extract_sync('report.pdf', config: config)
puts "Metadata: #{result.metadata.inspect}"
```
