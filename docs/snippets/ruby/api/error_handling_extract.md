```ruby title="Ruby"
require 'xberg'

begin
  pdf_bytes = File.read('document.pdf')
  config = Xberg::ExtractionConfig.new

  result = Xberg.extract_sync(pdf_bytes, 'application/pdf', config: config)
  puts "Extracted #{result.content.length} characters"
rescue RuntimeError => e
  # All extraction errors are raised as RuntimeError
  # Check error message for details
  case e.message
  when /parse|parsing/i
    puts "Failed to parse document: #{e.message}"
  when /ocr/i
    puts "OCR processing failed: #{e.message}"
  when /validation|invalid/i
    puts "Invalid configuration: #{e.message}"
  else
    puts "Extraction error: #{e.message}"
  end
end
```
