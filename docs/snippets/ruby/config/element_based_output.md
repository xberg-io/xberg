```ruby title="Element-Based Output (Ruby)"
require 'kreuzberg'

# Configure element-based output
config = Kreuzberg::ExtractionConfig.new(output_format: 'element_based')

# Extract document
result = Kreuzberg.extract_file_sync('document.pdf', config: config)

# Access elements
result.elements.each do |element|
  puts "Type: #{element.element_type}"
  puts "Text: #{element.text[0...100]}"

  puts "Page: #{element.metadata.page_number}" if element.metadata.page_number

  if element.metadata.coordinates
    coords = element.metadata.coordinates
    puts "Coords: (#{coords.left}, #{coords.top}) - (#{coords.right}, #{coords.bottom})"
  end

  puts "---"
end

# Filter by element type
titles = result.elements.select { |e| e.element_type == 'title' }
titles.each do |title|
  level = title.metadata.additional['level'] || 'unknown'
  puts "[#{level}] #{title.text}"
end
```
