```ruby title="Ruby"
require 'xberg'

input = Xberg::ExtractInput.new(uri: 'document.pdf')
config = Xberg::ExtractionConfig.new
result = Xberg.extract(input, config)

# Iterate over tables
result.results.first.tables.each do |table|
  puts "Table with #{table['cells'].length} rows"
  puts table['markdown']  # Markdown representation

  # Access cells
  table['cells'].each do |row|
    puts row
  end
end
```
