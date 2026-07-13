require "../src/xberg"

# Minimal config — only the fields we want to override from defaults.
# The Crystal struct uses `@[JSON::Field(default: ...)]` for all fields
# with Rust defaults, so partial JSON works.
config = Xberg::ExtractionConfig.from_json(%({"force_ocr":true}))
path = "../../test_documents/pdf_scanned/multi_page_scanned.pdf"
input = Xberg::ExtractInput.from_json(%({"kind":"Uri","uri":"#{path}"}))
puts "Extracting #{path} with tesseract OCR..."
result = Xberg.extract(input, config)
puts "Results: #{result.results.size}"
result.results.each_with_index { |r,i|
  puts "[#{i}] #{r.mime_type}: #{r.content[0..199]}"
}
