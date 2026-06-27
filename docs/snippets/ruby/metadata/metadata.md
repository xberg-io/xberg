```ruby title="Ruby"
require 'xberg'

input = Xberg::ExtractInput.new(uri: 'document.pdf')
config = Xberg::ExtractionConfig.new
result = Xberg.extract(input, config)

# Metadata is flat — format-specific fields are at the top level
metadata = result.results.first.metadata
if metadata['page_count']
  puts "Pages: #{metadata['page_count']}"
end
if metadata['title']
  puts "Title: #{metadata['title']}"
end
if metadata['authors']
  puts "Authors: #{metadata['authors'].join(', ')}"
end

# Access HTML metadata
html_input = Xberg::ExtractInput.new(uri: 'page.html')
html_result = Xberg.extract(html_input, Xberg::ExtractionConfig.new)
metadata = html_result.results.first.metadata
if metadata['title']
  puts "Title: #{metadata['title']}"
end
if metadata['description']
  puts "Description: #{metadata['description']}"
end

# Access keywords as array
if metadata['keywords']
  puts "Keywords: #{metadata['keywords'].join(', ')}"
end

# Access canonical URL (renamed from canonical)
puts "Canonical URL: #{metadata['canonical_url']}" if metadata['canonical_url']

# Access Open Graph fields from map
open_graph = metadata['open_graph'] || {}
puts "Open Graph Image: #{open_graph['image']}" if open_graph['image']
puts "Open Graph Title: #{open_graph['title']}" if open_graph['title']
puts "Open Graph Type: #{open_graph['type']}" if open_graph['type']

# Access Twitter Card fields from map
twitter_card = metadata['twitter_card'] || {}
puts "Twitter Card Type: #{twitter_card['card']}" if twitter_card['card']
puts "Twitter Creator: #{twitter_card['creator']}" if twitter_card['creator']

# Access new fields
puts "Language: #{metadata['language']}" if metadata['language']
puts "Text Direction: #{metadata['text_direction']}" if metadata['text_direction']

# Access headers
if metadata['headers']
  puts "Headers: #{metadata['headers'].map { |h| h['text'] }.join(', ')}"
end

# Access links
if metadata['links']
  metadata['links'].each do |link|
    puts "Link: #{link['href']} (#{link['text']})"
  end
end

# Access images
if metadata['images']
  metadata['images'].each do |image|
    puts "Image: #{image['src']}"
  end
end

# Access structured data
if metadata['structured_data']
  puts "Structured data items: #{metadata['structured_data'].length}"
end
```
