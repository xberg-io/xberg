```ruby title="Ruby"
require 'kreuzberg'

begin
  pdf_bytes = File.read('document.pdf')
  config = Kreuzberg::ExtractionConfig.new
  
  result = Kreuzberg.extract_bytes_sync(pdf_bytes, 'application/pdf', config: config)
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
