```ruby title="Ruby"
require 'kreuzberg'

class PdfOnlyProcessor
  def call(result)
    return result unless result['mime_type'] == 'application/pdf'
    result['metadata'] ||= {}
    result['metadata']['pdf_processed'] = true
    result
  end
end

processor = PdfOnlyProcessor.new
Kreuzberg.register_post_processor('pdf_only', processor)

config = Kreuzberg::ExtractionConfig.new(
  postprocessor: { enabled: true }
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)
puts "Metadata: #{result.metadata.inspect}"
```
