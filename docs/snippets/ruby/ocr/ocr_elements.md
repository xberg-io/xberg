```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(
    backend: 'paddleocr',
    language: 'eng'
  )
)

result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)

result.ocr_elements&.each do |element|
  puts "Text: #{element.text}"
  puts "Confidence: #{format('%.2f', element.confidence.recognition)}"
  puts "Geometry: #{element.geometry}"
  if element.rotation
    puts "Rotation: #{element.rotation.angle}°"
  end
  puts
end
```
